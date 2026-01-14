use alloy_primitives::{Address, U256};
use alloy_sol_types::SolCall;
use serde_json::Value;
use worker::kv::KvStore;
use worker::Env;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::infra::multicall::Call;
use crate::infra::token::Token;
use crate::types;

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
    let mut resp = worker::Fetch::Url(parsed_url)
        .send()
        .await
        .map_err(|err| CroLensError::RpcError(err.to_string()))?;

    let payload: Value = resp
        .json()
        .await
        .map_err(|err| CroLensError::RpcError(err.to_string()))?;

    for (symbol, id) in mapping {
        let price = payload
            .get(&id)
            .and_then(|v| v.get("usd"))
            .and_then(|v| v.as_f64());
        let Some(price_usd) = price else {
            continue;
        };

        let key = format!("price:anchor:{symbol}");
        kv.put(&key, price_usd.to_string())
            .map_err(|err| CroLensError::KvError(err.to_string()))?
            .expiration_ttl(600)
            .execute()
            .await
            .map_err(|err| CroLensError::KvError(err.to_string()))?;
    }

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
        .expiration_ttl(300)
        .execute()
        .await
        .map_err(|err| CroLensError::KvError(err.to_string()))?;

    Ok(Some(derived_price))
}
