use alloy_primitives::U256;
use alloy_sol_types::SolCall;
use serde::Deserialize;
use serde_json::Value;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::infra::multicall::Call;
use crate::types;

#[derive(Debug, Deserialize)]
struct GetTokenInfoArgs {
    token: String,
    #[serde(default)]
    simple_mode: bool,
}

/// Get detailed token information
pub async fn get_token_info(services: &infra::Services, args: Value) -> Result<Value> {
    let input: GetTokenInfoArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let token_query = input.token.trim();
    if token_query.is_empty() {
        return Err(CroLensError::invalid_params(
            "token must not be empty".to_string(),
        ));
    }

    // 1. Resolve token (address or symbol).
    let tokens = infra::token::list_tokens_cached(&services.db, &services.kv).await?;
    let token = infra::token::resolve_token(&tokens, token_query)?;

    // 2. Fetch on-chain metadata via multicall (name, symbol, decimals, totalSupply).
    let multicall = services.multicall()?;
    let calls = vec![
        Call {
            target: token.address,
            call_data: abi::nameCall {}.abi_encode().into(),
        },
        Call {
            target: token.address,
            call_data: abi::symbolCall {}.abi_encode().into(),
        },
        Call {
            target: token.address,
            call_data: abi::decimalsCall {}.abi_encode().into(),
        },
        Call {
            target: token.address,
            call_data: abi::totalSupplyCall {}.abi_encode().into(),
        },
    ];

    let results = multicall.aggregate(calls).await?;

    // Decode multicall return data.
    let name = results
        .get(0)
        .and_then(|r| r.as_ref().ok())
        .and_then(|data| abi::nameCall::abi_decode_returns(data, true).ok())
        .map(|v| v._0)
        .unwrap_or_else(|| token.symbol.clone());

    let symbol = results
        .get(1)
        .and_then(|r| r.as_ref().ok())
        .and_then(|data| abi::symbolCall::abi_decode_returns(data, true).ok())
        .map(|v| v._0)
        .unwrap_or_else(|| token.symbol.clone());

    let decimals = results
        .get(2)
        .and_then(|r| r.as_ref().ok())
        .and_then(|data| abi::decimalsCall::abi_decode_returns(data, true).ok())
        .map(|v| v._0)
        .unwrap_or(token.decimals);

    let total_supply = results
        .get(3)
        .and_then(|r| r.as_ref().ok())
        .and_then(|data| abi::totalSupplyCall::abi_decode_returns(data, true).ok())
        .map(|v| U256::from(v._0))
        .unwrap_or(U256::ZERO);

    let total_supply_formatted = types::format_units(&total_supply, decimals);

    // 3. Fetch token price (best-effort).
    let price_usd = infra::price::get_price_usd(services, &token)
        .await?
        .unwrap_or(0.0);

    // 4. Find main liquidity pools.
    let pools = infra::config::list_dex_pools_cached(&services.db, &services.kv, "vvs").await?;
    let token_pools: Vec<_> = pools
        .iter()
        .filter(|p| p.token0_address == token.address || p.token1_address == token.address)
        .collect();

    // Compute liquidity (requires pool reserves).
    let mut main_pools: Vec<Value> = Vec::new();
    let mut total_liquidity_usd = 0.0;

    if !token_pools.is_empty() {
        // Batch fetch reserves for all pools.
        let reserve_calls: Vec<Call> = token_pools
            .iter()
            .map(|pool| Call {
                target: pool.lp_address,
                call_data: abi::getReservesCall {}.abi_encode().into(),
            })
            .collect();

        let reserve_results = multicall.aggregate(reserve_calls).await?;

        // Fetch token prices for TVL estimation.
        let price_map = infra::price::get_prices_usd_batch(services, &tokens).await?;

        for (pool, result) in token_pools.iter().zip(reserve_results.into_iter()) {
            if let Ok(data) = result {
                if let Ok(decoded) = abi::getReservesCall::abi_decode_returns(&data, true) {
                    let reserve0 = U256::from(decoded.reserve0);
                    let reserve1 = U256::from(decoded.reserve1);

                    // Resolve token0/token1 metadata.
                    let token0 = tokens.iter().find(|t| t.address == pool.token0_address);
                    let token1 = tokens.iter().find(|t| t.address == pool.token1_address);

                    let token0_decimals = token0.map(|t| t.decimals).unwrap_or(18);
                    let token1_decimals = token1.map(|t| t.decimals).unwrap_or(18);

                    let amount0 = types::format_units(&reserve0, token0_decimals)
                        .parse::<f64>()
                        .unwrap_or(0.0);
                    let amount1 = types::format_units(&reserve1, token1_decimals)
                        .parse::<f64>()
                        .unwrap_or(0.0);

                    let price0 = price_map.get(&pool.token0_address).copied().unwrap_or(0.0);
                    let price1 = price_map.get(&pool.token1_address).copied().unwrap_or(0.0);

                    let tvl = amount0 * price0 + amount1 * price1;
                    total_liquidity_usd += tvl;

                    // Only include pools with TVL > $100.
                    if tvl > 100.0 {
                        main_pools.push(serde_json::json!({
                            "dex": "vvs",
                            "pair": format!("{}-{}", pool.token0_symbol, pool.token1_symbol),
                            "lp_address": pool.lp_address.to_string(),
                            "tvl_usd": format!("{:.2}", tvl)
                        }));
                    }
                }
            }
        }

        // Sort by TVL and keep top 5 pools.
        main_pools.sort_by(|a, b| {
            let tvl_a = a
                .get("tvl_usd")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let tvl_b = b
                .get("tvl_usd")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            tvl_b.partial_cmp(&tvl_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        main_pools.truncate(5);
    }

    // 5. Compute market cap (if price is available).
    let total_supply_f64 = total_supply_formatted.parse::<f64>().unwrap_or(0.0);
    let market_cap_usd = if price_usd > 0.0 && total_supply_f64 > 0.0 {
        Some(price_usd * total_supply_f64)
    } else {
        None
    };

    // 6. Build response.
    if input.simple_mode {
        let mcap_str = market_cap_usd
            .map(|v| format_currency(v))
            .unwrap_or_else(|| "N/A".to_string());
        let liq_str = if total_liquidity_usd > 0.0 {
            format_currency(total_liquidity_usd)
        } else {
            "N/A".to_string()
        };
        let pool_hint = main_pools
            .first()
            .and_then(|p| p.get("pair"))
            .and_then(|v| v.as_str())
            .map(|s| format!(" ({} pool)", s))
            .unwrap_or_default();

        let text = format!(
            "{} ({}) | Price: ${:.6} | MCap: {} | Liquidity: {}{}",
            name, symbol, price_usd, mcap_str, liq_str, pool_hint
        );
        return Ok(serde_json::json!({ "text": text }));
    }

    Ok(serde_json::json!({
        "address": token.address.to_string(),
        "name": name,
        "symbol": symbol,
        "decimals": decimals,
        "total_supply": total_supply_formatted,
        "price_usd": format!("{:.8}", price_usd),
        "market_cap_usd": market_cap_usd.map(|v| format!("{:.2}", v)),
        "liquidity_usd": format!("{:.2}", total_liquidity_usd),
        "main_pools": main_pools,
        "meta": services.meta()
    }))
}

/// Format currency with K/M/B suffixes.
fn format_currency(value: f64) -> String {
    if value >= 1_000_000_000.0 {
        format!("${:.2}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("${:.2}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("${:.2}K", value / 1_000.0)
    } else {
        format!("${:.2}", value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_currency_scales() {
        assert_eq!(format_currency(999.0), "$999.00");
        assert_eq!(format_currency(1_000.0), "$1.00K");
        assert_eq!(format_currency(1_000_000.0), "$1.00M");
        assert_eq!(format_currency(1_000_000_000.0), "$1.00B");
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({ "token": "VVS" });
        let args: GetTokenInfoArgs = serde_json::from_value(json).expect("args should parse");
        assert_eq!(args.token, "VVS");
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({ "token": "VVS", "simple_mode": true });
        let args: GetTokenInfoArgs = serde_json::from_value(json).expect("args should parse");
        assert!(args.simple_mode);
    }
}
