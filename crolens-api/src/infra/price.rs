use std::collections::HashMap;

use alloy_primitives::{Address, U256};
use alloy_sol_types::SolCall;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use worker::kv::KvStore;
use worker::Env;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::infra::multicall::Call;
use crate::infra::token::Token;
use crate::types;

/// 所有价格的聚合缓存 key
const ALL_PRICES_CACHE_KEY: &str = "cache:prices:all";

/// 价格缓存结构
#[derive(Serialize, Deserialize)]
struct PriceCache {
    // address (lowercase) -> price_usd
    prices: HashMap<String, f64>,
}

/// 批量获取多个代币的 USD 价格
/// 优化：从单个 KV key 读取所有价格，而不是多次 KV 查询
pub async fn get_prices_usd_batch(
    services: &infra::Services,
    tokens: &[Token],
) -> Result<HashMap<Address, f64>> {
    let mut result = HashMap::with_capacity(tokens.len());

    // 1. 先处理稳定币
    for token in tokens {
        if token.is_stablecoin {
            result.insert(token.address, 1.0);
        }
    }

    // 2. 尝试从聚合缓存读取所有价格 (单次 KV 读取)
    let t0 = crate::types::now_ms();
    if let Ok(Some(cached)) = services.kv.get(ALL_PRICES_CACHE_KEY).text().await {
        let t1 = crate::types::now_ms();
        if let Ok(cache) = serde_json::from_str::<PriceCache>(&cached) {
            for token in tokens {
                if result.contains_key(&token.address) {
                    continue; // 已经是稳定币
                }
                let addr_key = token.address.to_string().to_lowercase();
                if let Some(&price) = cache.prices.get(&addr_key) {
                    result.insert(token.address, price);
                }
            }
            // 如果所有代币都找到了价格，直接返回
            if result.len() == tokens.len() {
                worker::console_log!("[PERF] price cache HIT: {}ms, {} prices", t1 - t0, result.len());
                return Ok(result);
            }
            worker::console_log!("[PERF] price cache PARTIAL: {}ms, {}/{} prices", t1 - t0, result.len(), tokens.len());
        }
    } else {
        let t1 = crate::types::now_ms();
        worker::console_log!("[PERF] price cache MISS: {}ms", t1 - t0);
    }

    // 3. 聚合缓存未命中或不完整，回退到原来的多次 KV 查询
    let mut anchor_queries: Vec<(Address, String)> = Vec::new();
    let mut derived_queries: Vec<(Address, String)> = Vec::new();

    for token in tokens {
        if result.contains_key(&token.address) {
            continue;
        }
        anchor_queries.push((token.address, normalize_anchor_symbol(&token.symbol)));
        let addr_key = token.address.to_string().to_lowercase();
        derived_queries.push((token.address, format!("price:derived:{addr_key}")));
    }

    // 并行查询所有 anchor 价格
    let anchor_futures = anchor_queries.iter().map(|(_, symbol)| {
        let key = format!("price:anchor:{symbol}");
        let kv = &services.kv;
        async move {
            kv.get(&key)
                .text()
                .await
                .ok()
                .flatten()
                .and_then(|t| t.parse::<f64>().ok())
        }
    });

    let anchor_results: Vec<Option<f64>> =
        futures_util::future::join_all(anchor_futures).await;

    for ((addr, _), price) in anchor_queries.iter().zip(anchor_results.into_iter()) {
        if let Some(p) = price {
            result.insert(*addr, p);
        }
    }

    // 对于还没找到价格的代币，查询 derived 缓存
    let derived_futures = derived_queries.iter().map(|(addr, key)| {
        let already_found = result.contains_key(addr);
        let key = key.clone();
        let kv = &services.kv;
        async move {
            if already_found {
                return None;
            }
            kv.get(&key)
                .text()
                .await
                .ok()
                .flatten()
                .and_then(|t| t.parse::<f64>().ok())
        }
    });

    let derived_results: Vec<Option<f64>> =
        futures_util::future::join_all(derived_futures).await;

    for ((addr, _), price) in derived_queries.iter().zip(derived_results.into_iter()) {
        if !result.contains_key(addr) {
            if let Some(p) = price {
                result.insert(*addr, p);
            }
        }
    }

    Ok(result)
}

pub async fn get_price_usd(services: &infra::Services, token: &Token) -> Result<Option<f64>> {
    if token.is_stablecoin {
        return Ok(Some(1.0));
    }

    if let Some(anchor) = get_anchor_price_usd(&services.kv, &token.symbol).await? {
        return Ok(Some(anchor));
    }

    let addr_key = token.address.to_string().to_lowercase();
    let derived_key = format!("price:derived:{addr_key}");
    if let Some(text) = services
        .kv
        .get(&derived_key)
        .text()
        .await
        .map_err(|err| CroLensError::KvError(err.to_string()))?
    {
        let parsed = text.parse::<f64>().map_err(|err| {
            CroLensError::KvError(format!("Invalid KV price for {derived_key}: {err}"))
        })?;
        return Ok(Some(parsed));
    }

    derive_price_from_pool(services, token.address).await
}

pub async fn update_anchor_prices(env: &Env) -> Result<()> {
    let db = env
        .d1("DB")
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let kv = env
        .kv("KV")
        .map_err(|err| CroLensError::KvError(err.to_string()))?;

    let statement = db.prepare(
        "SELECT symbol, coingecko_id FROM tokens WHERE is_anchor = 1 AND coingecko_id IS NOT NULL",
    );
    let result = infra::db::run("update_anchor_prices_select", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let mut ids: Vec<String> = Vec::new();
    let mut mapping: Vec<(String, String)> = Vec::new();
    for row in rows {
        let symbol = row
            .get("symbol")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CroLensError::DbError("tokens.symbol missing".to_string()))?;
        let coingecko_id = row
            .get("coingecko_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CroLensError::DbError("tokens.coingecko_id missing".to_string()))?;
        ids.push(coingecko_id.to_string());
        mapping.push((normalize_anchor_symbol(symbol), coingecko_id.to_string()));
    }

    if ids.is_empty() {
        return Ok(());
    }

    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
        ids.join("%2C")
    );
    let parsed_url =
        worker::Url::parse(&url).map_err(|err| CroLensError::RpcError(err.to_string()))?;

    let mut headers = worker::Headers::new();
    headers
        .set("User-Agent", "CroLens/1.0 (https://crolens.io)")
        .map_err(|err| CroLensError::RpcError(err.to_string()))?;
    headers
        .set("Accept", "application/json")
        .map_err(|err| CroLensError::RpcError(err.to_string()))?;

    let req = worker::Request::new_with_init(
        url.as_str(),
        worker::RequestInit::new()
            .with_method(worker::Method::Get)
            .with_headers(headers),
    )
    .map_err(|err| CroLensError::RpcError(err.to_string()))?;

    let mut resp = worker::Fetch::Request(req)
        .send()
        .await
        .map_err(|err| CroLensError::RpcError(err.to_string()))?;

    let payload: Value = resp
        .json()
        .await
        .map_err(|err| CroLensError::RpcError(err.to_string()))?;

    worker::console_log!("[DEBUG] CoinGecko response: {}", payload.to_string());

    let mut write_count = 0;
    for (symbol, id) in mapping {
        let price = payload
            .get(&id)
            .and_then(|v| v.get("usd"))
            .and_then(|v| v.as_f64());
        let Some(price_usd) = price else {
            worker::console_log!("[DEBUG] No price for {} (id: {})", symbol, id);
            continue;
        };

        let key = format!("price:anchor:{symbol}");
        worker::console_log!("[DEBUG] Writing anchor price: {} = {}", key, price_usd);
        kv.put(&key, price_usd.to_string())
            .map_err(|err| CroLensError::KvError(err.to_string()))?
            .expiration_ttl(900) // 15 分钟，比 cron 间隔 (5分钟) 长，确保缓存不会过期
            .execute()
            .await
            .map_err(|err| CroLensError::KvError(err.to_string()))?;
        write_count += 1;
    }

    worker::console_log!("[DEBUG] Wrote {} anchor prices", write_count);
    Ok(())
}

/// 预热所有非 anchor 代币的 derived 价格
/// 在 scheduled worker 中调用，将所有代币价格提前计算并缓存到 KV
/// 同时写入聚合缓存 (ALL_PRICES_CACHE_KEY) 供 get_prices_usd_batch 使用
pub async fn update_derived_prices(env: &Env) -> Result<()> {
    let db = env
        .d1("DB")
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let kv = env
        .kv("KV")
        .map_err(|err| CroLensError::KvError(err.to_string()))?;

    // 聚合价格缓存：收集所有价格
    let mut all_prices: HashMap<String, f64> = HashMap::new();

    // 1. 获取所有 anchor 代币价格
    let anchor_stmt = db.prepare(
        "SELECT address, symbol FROM tokens WHERE is_anchor = 1",
    );
    let anchor_result = infra::db::run("update_derived_anchor_select", anchor_stmt.all()).await?;
    let anchor_rows: Vec<Value> = anchor_result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    for row in &anchor_rows {
        let address_str = match row.get("address").and_then(|v| v.as_str()) {
            Some(v) => v,
            None => continue,
        };
        let symbol = match row.get("symbol").and_then(|v| v.as_str()) {
            Some(v) => v,
            None => continue,
        };
        if let Some(price) = get_anchor_price_usd(&kv, symbol).await.ok().flatten() {
            all_prices.insert(address_str.to_lowercase(), price);
        }
    }

    // 2. 获取所有稳定币
    let stable_stmt = db.prepare("SELECT address FROM tokens WHERE is_stablecoin = 1");
    let stable_result = infra::db::run("update_derived_stable_select", stable_stmt.all()).await?;
    let stable_rows: Vec<Value> = stable_result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    for row in &stable_rows {
        if let Some(addr) = row.get("address").and_then(|v| v.as_str()) {
            all_prices.insert(addr.to_lowercase(), 1.0);
        }
    }

    // 3. 获取所有非 anchor、非稳定币的代币
    let statement = db.prepare(
        "SELECT address, symbol, decimals FROM tokens WHERE is_anchor = 0 AND is_stablecoin = 0",
    );
    let result = infra::db::run("update_derived_prices_select", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    if rows.is_empty() {
        // 仍然写入聚合缓存（包含 anchor 和 stablecoin）
        write_aggregated_price_cache(&kv, &all_prices).await?;
        return Ok(());
    }

    // 构建 Services (需要 RPC)
    let services = infra::Services::new(env, "cron:derived_prices", types::now_ms())?;
    let multicall = services.multicall()?;

    // 获取所有 DEX 池子信息
    let pools = infra::config::list_dex_pools(&db, "vvs").await?;
    if pools.is_empty() {
        write_aggregated_price_cache(&kv, &all_prices).await?;
        return Ok(());
    }

    // 批量获取所有池子的 reserves (用一次 multicall)
    let reserve_calls: Vec<Call> = pools
        .iter()
        .map(|pool| Call {
            target: pool.lp_address,
            call_data: abi::getReservesCall {}.abi_encode().into(),
        })
        .collect();

    let reserve_results = multicall.aggregate(reserve_calls).await?;

    // 解析 reserves 结果并建立映射
    let mut pool_reserves: std::collections::HashMap<
        alloy_primitives::Address,
        (U256, U256, Address, Address),
    > = std::collections::HashMap::new();

    for (pool, result) in pools.iter().zip(reserve_results.into_iter()) {
        if let Ok(data) = result {
            if let Ok(decoded) = abi::getReservesCall::abi_decode_returns(&data, true) {
                pool_reserves.insert(
                    pool.lp_address,
                    (
                        U256::from(decoded.reserve0),
                        U256::from(decoded.reserve1),
                        pool.token0_address,
                        pool.token1_address,
                    ),
                );
            }
        }
    }

    // 获取所有代币信息用于 decimals 查询
    let all_tokens = infra::token::list_tokens(&db).await?;
    let token_decimals: std::collections::HashMap<Address, u8> = all_tokens
        .iter()
        .map(|t| (t.address, t.decimals))
        .collect();
    let token_symbols: std::collections::HashMap<Address, String> = all_tokens
        .iter()
        .map(|t| (t.address, t.symbol.clone()))
        .collect();

    // 对每个需要计算 derived price 的代币
    for row in rows {
        let address_str = match row.get("address").and_then(|v| v.as_str()) {
            Some(v) => v,
            None => continue,
        };
        let token_address = match types::parse_address(address_str) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let _token_decimals_val = token_decimals.get(&token_address).copied().unwrap_or(18);

        // 查找该代币所在的池子
        let pool = pools.iter().find(|p| {
            p.token0_address == token_address || p.token1_address == token_address
        });
        let Some(pool) = pool else {
            continue;
        };

        let Some((reserve0, reserve1, token0_addr, token1_addr)) =
            pool_reserves.get(&pool.lp_address)
        else {
            continue;
        };

        let token0_dec = token_decimals.get(token0_addr).copied().unwrap_or(18);
        let token1_dec = token_decimals.get(token1_addr).copied().unwrap_or(18);

        let token0_amount = types::format_units(reserve0, token0_dec)
            .parse::<f64>()
            .unwrap_or(0.0);
        let token1_amount = types::format_units(reserve1, token1_dec)
            .parse::<f64>()
            .unwrap_or(0.0);

        let (token_amount, quote_amount, quote_symbol) = if token_address == *token0_addr {
            let sym = token_symbols
                .get(token1_addr)
                .map(|s| s.as_str())
                .unwrap_or("UNKNOWN");
            (token0_amount, token1_amount, sym)
        } else {
            let sym = token_symbols
                .get(token0_addr)
                .map(|s| s.as_str())
                .unwrap_or("UNKNOWN");
            (token1_amount, token0_amount, sym)
        };

        if token_amount <= 0.0 || quote_amount <= 0.0 {
            continue;
        }

        // 获取 quote token 的价格
        let quote_price_usd = if quote_symbol.eq_ignore_ascii_case("USDC")
            || quote_symbol.eq_ignore_ascii_case("USDT")
        {
            Some(1.0)
        } else {
            get_anchor_price_usd(&kv, quote_symbol).await.ok().flatten()
        };

        let Some(quote_price) = quote_price_usd else {
            continue;
        };

        let derived_price = quote_price * (quote_amount / token_amount);
        if !derived_price.is_finite() || derived_price <= 0.0 {
            continue;
        }

        // 写入单独的 KV 缓存 (兼容旧逻辑)
        let addr_key = token_address.to_string().to_lowercase();
        let key = format!("price:derived:{addr_key}");
        if let Ok(put) = kv.put(&key, derived_price.to_string()) {
            let _ = put.expiration_ttl(600).execute().await;
        }

        // 添加到聚合缓存
        all_prices.insert(addr_key, derived_price);
    }

    // 写入聚合价格缓存
    write_aggregated_price_cache(&kv, &all_prices).await?;

    Ok(())
}

/// 写入聚合价格缓存
async fn write_aggregated_price_cache(kv: &KvStore, prices: &HashMap<String, f64>) -> Result<()> {
    let cache = PriceCache {
        prices: prices.clone(),
    };
    let json = serde_json::to_string(&cache)
        .map_err(|err| CroLensError::KvError(format!("Failed to serialize price cache: {err}")))?;

    kv.put(ALL_PRICES_CACHE_KEY, json)
        .map_err(|err| CroLensError::KvError(err.to_string()))?
        .expiration_ttl(600) // 10 分钟
        .execute()
        .await
        .map_err(|err| CroLensError::KvError(err.to_string()))?;

    Ok(())
}

async fn get_anchor_price_usd(kv: &KvStore, symbol: &str) -> Result<Option<f64>> {
    let key_symbol = normalize_anchor_symbol(symbol);
    let key = format!("price:anchor:{key_symbol}");
    let value = kv
        .get(&key)
        .text()
        .await
        .map_err(|err| CroLensError::KvError(err.to_string()))?;

    let Some(text) = value else {
        return Ok(None);
    };

    text.parse::<f64>()
        .map(Some)
        .map_err(|err| CroLensError::KvError(format!("Invalid KV price for {key}: {err}")))
}

fn normalize_anchor_symbol(symbol: &str) -> String {
    let normalized = types::normalize_symbol(symbol);
    if normalized == "wcro" || normalized == "cro" {
        return "cro".to_string();
    }
    normalized
}

async fn derive_price_from_pool(
    services: &infra::Services,
    token_address: Address,
) -> Result<Option<f64>> {
    let rpc_pool = services.multicall()?;
    let Some(pool) = infra::config::find_pool_for_token(&services.db, token_address).await? else {
        return Ok(None);
    };

    let reserve_call = Call {
        target: pool.lp_address,
        call_data: abi::getReservesCall {}.abi_encode().into(),
    };
    let reserves = rpc_pool.aggregate(vec![reserve_call]).await?;
    let Some(item) = reserves.into_iter().next() else {
        return Ok(None);
    };
    let Ok(return_data) = item else {
        return Ok(None);
    };

    let decoded = abi::getReservesCall::abi_decode_returns(&return_data, true)
        .map_err(|err| CroLensError::RpcError(format!("getReserves decode failed: {err}")))?;

    let token0 = infra::token::get_token_by_address(&services.db, pool.token0_address).await?;
    let token1 = infra::token::get_token_by_address(&services.db, pool.token1_address).await?;

    let token0_decimals = token0.as_ref().map(|t| t.decimals).unwrap_or(18);
    let token1_decimals = token1.as_ref().map(|t| t.decimals).unwrap_or(18);

    let reserve0 = U256::from(decoded.reserve0);
    let reserve1 = U256::from(decoded.reserve1);

    let token0_amount = types::format_units(&reserve0, token0_decimals)
        .parse::<f64>()
        .unwrap_or(0.0);
    let token1_amount = types::format_units(&reserve1, token1_decimals)
        .parse::<f64>()
        .unwrap_or(0.0);

    let (token_amount, quote_amount, quote_symbol) = if token_address == pool.token0_address {
        let sym = token1
            .as_ref()
            .map(|t| t.symbol.as_str())
            .unwrap_or("UNKNOWN");
        (token0_amount, token1_amount, sym)
    } else if token_address == pool.token1_address {
        let sym = token0
            .as_ref()
            .map(|t| t.symbol.as_str())
            .unwrap_or("UNKNOWN");
        (token1_amount, token0_amount, sym)
    } else {
        return Ok(None);
    };

    if token_amount <= 0.0 || quote_amount <= 0.0 {
        return Ok(None);
    }

    let quote_price_usd =
        if quote_symbol.eq_ignore_ascii_case("USDC") || quote_symbol.eq_ignore_ascii_case("USDT") {
            Some(1.0)
        } else {
            get_anchor_price_usd(&services.kv, quote_symbol).await?
        };

    let Some(quote_price) = quote_price_usd else {
        return Ok(None);
    };

    let derived_price = quote_price * (quote_amount / token_amount);
    if !derived_price.is_finite() || derived_price <= 0.0 {
        return Ok(None);
    }

    let addr_key = token_address.to_string().to_lowercase();
    let key = format!("price:derived:{addr_key}");
    services
        .kv
        .put(&key, derived_price.to_string())
        .map_err(|err| CroLensError::KvError(err.to_string()))?
        .expiration_ttl(600) // 10 分钟，比 cron 间隔 (5分钟) 长
        .execute()
        .await
        .map_err(|err| CroLensError::KvError(err.to_string()))?;

    Ok(Some(derived_price))
}
