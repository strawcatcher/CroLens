use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use worker::d1::D1Type;
use worker::kv::KvStore;
use worker::D1Database;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

const DEX_POOLS_CACHE_PREFIX: &str = "cache:dex_pools:";
const LENDING_MARKETS_CACHE_PREFIX: &str = "cache:lending_markets:";
const CONFIG_CACHE_TTL_SECS: u64 = 600; // 10 分钟

#[derive(Debug, Clone)]
pub struct DexPool {
    pub pool_id: String,
    pub pool_index: Option<i64>,
    pub lp_address: Address,
    pub token0_address: Address,
    pub token1_address: Address,
    pub token0_symbol: String,
    pub token1_symbol: String,
}

#[derive(Debug, Clone)]
pub struct LendingMarket {
    pub ctoken_address: Address,
    pub underlying_address: Address,
    pub underlying_symbol: String,
    pub collateral_factor: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct DexPoolCache {
    pool_id: String,
    pool_index: Option<i64>,
    lp_address: String,
    token0_address: String,
    token1_address: String,
    token0_symbol: String,
    token1_symbol: String,
}

#[derive(Serialize, Deserialize)]
struct LendingMarketCache {
    ctoken_address: String,
    underlying_address: String,
    underlying_symbol: String,
    collateral_factor: Option<String>,
}

pub async fn get_protocol_contract(
    db: &D1Database,
    protocol_id: &str,
    contract_type: &str,
) -> Result<Address> {
    let protocol_arg = D1Type::Text(protocol_id);
    let contract_arg = D1Type::Text(contract_type);
    let statement = db
        .prepare(
            "SELECT address FROM protocol_contracts \
             WHERE protocol_id = ?1 AND contract_type = ?2 AND chain_id = 25 LIMIT 1",
        )
        .bind_refs([&protocol_arg, &contract_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let result = infra::db::run("get_protocol_contract", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let Some(row) = rows.first() else {
        return Err(CroLensError::DbError(format!(
            "Missing protocol contract: {protocol_id}.{contract_type}"
        )));
    };

    let address = row
        .get("address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CroLensError::DbError("protocol_contracts.address missing".to_string()))?;
    types::parse_address(address)
}

/// 从 KV 缓存获取 DEX 池子列表
pub async fn list_dex_pools_cached(
    db: &D1Database,
    kv: &KvStore,
    protocol_id: &str,
) -> Result<Vec<DexPool>> {
    let cache_key = format!("{DEX_POOLS_CACHE_PREFIX}{protocol_id}");

    // 先尝试从 KV 缓存获取
    if let Ok(Some(cached)) = kv.get(&cache_key).text().await {
        if let Ok(pools_cache) = serde_json::from_str::<Vec<DexPoolCache>>(&cached) {
            let mut pools = Vec::with_capacity(pools_cache.len());
            for p in pools_cache {
                if let (Ok(lp), Ok(t0), Ok(t1)) = (
                    types::parse_address(&p.lp_address),
                    types::parse_address(&p.token0_address),
                    types::parse_address(&p.token1_address),
                ) {
                    pools.push(DexPool {
                        pool_id: p.pool_id,
                        pool_index: p.pool_index,
                        lp_address: lp,
                        token0_address: t0,
                        token1_address: t1,
                        token0_symbol: p.token0_symbol,
                        token1_symbol: p.token1_symbol,
                    });
                }
            }
            if !pools.is_empty() {
                return Ok(pools);
            }
        }
    }

    // 缓存未命中，从 DB 加载
    let pools = list_dex_pools(db, protocol_id).await?;

    // 写入缓存
    let cache: Vec<DexPoolCache> = pools
        .iter()
        .map(|p| DexPoolCache {
            pool_id: p.pool_id.clone(),
            pool_index: p.pool_index,
            lp_address: p.lp_address.to_string(),
            token0_address: p.token0_address.to_string(),
            token1_address: p.token1_address.to_string(),
            token0_symbol: p.token0_symbol.clone(),
            token1_symbol: p.token1_symbol.clone(),
        })
        .collect();
    if let Ok(json) = serde_json::to_string(&cache) {
        if let Ok(put) = kv.put(&cache_key, json) {
            let _ = put.expiration_ttl(CONFIG_CACHE_TTL_SECS).execute().await;
        }
    }

    Ok(pools)
}

pub async fn list_dex_pools(db: &D1Database, protocol_id: &str) -> Result<Vec<DexPool>> {
    let protocol_arg = D1Type::Text(protocol_id);
    let statement = db
        .prepare(
            "SELECT pool_id, pool_index, lp_address, token0_address, token1_address, token0_symbol, token1_symbol \
             FROM dex_pools WHERE protocol_id = ?1 AND is_active = 1",
        )
        .bind_refs([&protocol_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let result = infra::db::run("list_dex_pools", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let mut pools = Vec::with_capacity(rows.len());
    for row in rows {
        let pool_id = row
            .get("pool_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CroLensError::DbError("dex_pools.pool_id missing".to_string()))?
            .to_string();
        let pool_index = row.get("pool_index").and_then(|v| v.as_i64());
        let lp_address = row
            .get("lp_address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CroLensError::DbError("dex_pools.lp_address missing".to_string()))?;
        let token0_address = row
            .get("token0_address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CroLensError::DbError("dex_pools.token0_address missing".to_string()))?;
        let token1_address = row
            .get("token1_address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CroLensError::DbError("dex_pools.token1_address missing".to_string()))?;

        let token0_symbol = row
            .get("token0_symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("TOKEN0")
            .to_string();
        let token1_symbol = row
            .get("token1_symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("TOKEN1")
            .to_string();

        pools.push(DexPool {
            pool_id,
            pool_index,
            lp_address: types::parse_address(lp_address)?,
            token0_address: types::parse_address(token0_address)?,
            token1_address: types::parse_address(token1_address)?,
            token0_symbol,
            token1_symbol,
        });
    }

    Ok(pools)
}

pub async fn find_pool_for_token(
    db: &D1Database,
    token_address: Address,
) -> Result<Option<DexPool>> {
    let Some(wcro) = get_token_address_by_symbol(db, "WCRO").await? else {
        return Ok(None);
    };
    let Some(usdc) = get_token_address_by_symbol(db, "USDC").await? else {
        return Ok(None);
    };

    if let Some(pool) = find_pool_for_pair(db, "vvs", token_address, wcro).await? {
        return Ok(Some(pool));
    }
    if let Some(pool) = find_pool_for_pair(db, "vvs", token_address, usdc).await? {
        return Ok(Some(pool));
    }

    Ok(None)
}

pub async fn get_token_address_by_symbol(db: &D1Database, symbol: &str) -> Result<Option<Address>> {
    let symbol_normalized = symbol.trim().to_lowercase();
    let symbol_arg = D1Type::Text(&symbol_normalized);
    let statement = db
        .prepare("SELECT address FROM tokens WHERE lower(symbol) = ?1 LIMIT 1")
        .bind_refs([&symbol_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let result = infra::db::run("get_token_address_by_symbol", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let Some(row) = rows.first() else {
        return Ok(None);
    };

    let address = row
        .get("address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CroLensError::DbError("tokens.address missing".to_string()))?;
    Ok(Some(types::parse_address(address)?))
}

async fn find_pool_for_pair(
    db: &D1Database,
    protocol_id: &str,
    token_a: Address,
    token_b: Address,
) -> Result<Option<DexPool>> {
    let protocol_arg = D1Type::Text(protocol_id);
    let token_a_str = token_a.to_string();
    let token_b_str = token_b.to_string();
    let token_a_arg = D1Type::Text(&token_a_str);
    let token_b_arg = D1Type::Text(&token_b_str);

    let statement = db
        .prepare(
            "SELECT pool_id, pool_index, lp_address, token0_address, token1_address, token0_symbol, token1_symbol \
             FROM dex_pools \
             WHERE protocol_id = ?1 AND is_active = 1 AND ((token0_address = ?2 AND token1_address = ?3) OR (token0_address = ?3 AND token1_address = ?2)) \
             LIMIT 1",
        )
        .bind_refs([&protocol_arg, &token_a_arg, &token_b_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let result = infra::db::run("find_pool_for_pair", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let Some(row) = rows.first() else {
        return Ok(None);
    };

    let pool_id = row
        .get("pool_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CroLensError::DbError("dex_pools.pool_id missing".to_string()))?
        .to_string();
    let pool_index = row.get("pool_index").and_then(|v| v.as_i64());
    let lp_address = row
        .get("lp_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CroLensError::DbError("dex_pools.lp_address missing".to_string()))?;
    let token0_address = row
        .get("token0_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CroLensError::DbError("dex_pools.token0_address missing".to_string()))?;
    let token1_address = row
        .get("token1_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CroLensError::DbError("dex_pools.token1_address missing".to_string()))?;
    let token0_symbol = row
        .get("token0_symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("TOKEN0")
        .to_string();
    let token1_symbol = row
        .get("token1_symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("TOKEN1")
        .to_string();

    Ok(Some(DexPool {
        pool_id,
        pool_index,
        lp_address: types::parse_address(lp_address)?,
        token0_address: types::parse_address(token0_address)?,
        token1_address: types::parse_address(token1_address)?,
        token0_symbol,
        token1_symbol,
    }))
}

/// 从 KV 缓存获取 Lending markets 列表
pub async fn list_lending_markets_cached(
    db: &D1Database,
    kv: &KvStore,
    protocol_id: &str,
) -> Result<Vec<LendingMarket>> {
    let cache_key = format!("{LENDING_MARKETS_CACHE_PREFIX}{protocol_id}");

    // 先尝试从 KV 缓存获取
    if let Ok(Some(cached)) = kv.get(&cache_key).text().await {
        if let Ok(markets_cache) = serde_json::from_str::<Vec<LendingMarketCache>>(&cached) {
            let mut markets = Vec::with_capacity(markets_cache.len());
            for m in markets_cache {
                if let (Ok(ctoken), Ok(underlying)) = (
                    types::parse_address(&m.ctoken_address),
                    types::parse_address(&m.underlying_address),
                ) {
                    markets.push(LendingMarket {
                        ctoken_address: ctoken,
                        underlying_address: underlying,
                        underlying_symbol: m.underlying_symbol,
                        collateral_factor: m.collateral_factor,
                    });
                }
            }
            if !markets.is_empty() {
                return Ok(markets);
            }
        }
    }

    // 缓存未命中，从 DB 加载
    let markets = list_lending_markets(db, protocol_id).await?;

    // 写入缓存
    let cache: Vec<LendingMarketCache> = markets
        .iter()
        .map(|m| LendingMarketCache {
            ctoken_address: m.ctoken_address.to_string(),
            underlying_address: m.underlying_address.to_string(),
            underlying_symbol: m.underlying_symbol.clone(),
            collateral_factor: m.collateral_factor.clone(),
        })
        .collect();
    if let Ok(json) = serde_json::to_string(&cache) {
        if let Ok(put) = kv.put(&cache_key, json) {
            let _ = put.expiration_ttl(CONFIG_CACHE_TTL_SECS).execute().await;
        }
    }

    Ok(markets)
}

pub async fn list_lending_markets(
    db: &D1Database,
    protocol_id: &str,
) -> Result<Vec<LendingMarket>> {
    let protocol_arg = D1Type::Text(protocol_id);
    let statement = db
        .prepare(
            "SELECT ctoken_address, underlying_address, underlying_symbol, collateral_factor \
             FROM lending_markets WHERE protocol_id = ?1 AND is_active = 1",
        )
        .bind_refs([&protocol_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let result = infra::db::run("list_lending_markets", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let mut markets = Vec::with_capacity(rows.len());
    for row in rows {
        let ctoken_address = row
            .get("ctoken_address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                CroLensError::DbError("lending_markets.ctoken_address missing".to_string())
            })?;
        let underlying_address = row
            .get("underlying_address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                CroLensError::DbError("lending_markets.underlying_address missing".to_string())
            })?;
        let underlying_symbol = row
            .get("underlying_symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("UNDERLYING")
            .to_string();
        let collateral_factor = row
            .get("collateral_factor")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());

        markets.push(LendingMarket {
            ctoken_address: types::parse_address(ctoken_address)?,
            underlying_address: types::parse_address(underlying_address)?,
            underlying_symbol,
            collateral_factor,
        });
    }

    Ok(markets)
}
