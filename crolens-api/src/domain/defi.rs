use alloy_primitives::U256;
use alloy_sol_types::SolCall;
use serde::Deserialize;
use serde_json::Value;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

const BLOCKS_PER_YEAR: f64 = 179_740_800.0;
const VVS_MASTERCHEF_ADDRESS: &str = "0x3790f3A1cf8A478042Ec112A70881Dcfa9c0fc21";

#[derive(Debug, Deserialize)]
struct GetDefiPositionsArgs {
    address: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_defi_positions(services: &infra::Services, args: Value) -> Result<Value> {
    let input: GetDefiPositionsArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;
    let user = types::parse_address(&input.address)?;

    let pools = infra::config::list_dex_pools(&services.db, "vvs").await?;
    let masterchef =
        match infra::config::get_protocol_contract(&services.db, "vvs", "masterchef").await {
            Ok(addr) => addr,
            Err(_) => types::parse_address(VVS_MASTERCHEF_ADDRESS)?,
        };
    let mut calls = Vec::with_capacity(pools.len() * 5);
    for pool in &pools {
        calls.push(infra::multicall::Call {
            target: pool.lp_address,
            call_data: abi::balanceOfCall { account: user }.abi_encode().into(),
        });
        calls.push(infra::multicall::Call {
            target: pool.lp_address,
            call_data: abi::getReservesCall {}.abi_encode().into(),
        });
        calls.push(infra::multicall::Call {
            target: pool.lp_address,
            call_data: abi::totalSupplyCall {}.abi_encode().into(),
        });

        if let Some(pid) = pool.pool_index {
            calls.push(infra::multicall::Call {
                target: masterchef,
                call_data: abi::userInfoCall {
                    pid: U256::from(pid as u64),
                    user,
                }
                .abi_encode()
                .into(),
            });
            calls.push(infra::multicall::Call {
                target: masterchef,
                call_data: abi::pendingVVSCall {
                    pid: U256::from(pid as u64),
                    user,
                }
                .abi_encode()
                .into(),
            });
        }
    }

    let results = services.multicall()?.aggregate(calls).await?;
    let mut results_iter = results.into_iter();
    let mut vvs_positions: Vec<Value> = Vec::new();
    let mut vvs_total_liquidity_usd = 0.0_f64;
    let mut vvs_total_pending_rewards_usd = 0.0_f64;
    let mut vvs_total_pending_vvs: U256 = U256::ZERO;

    let tokens = infra::token::list_tokens(&services.db).await?;
    let token_map = tokens;
    let vvs_token = token_map
        .iter()
        .find(|t| t.symbol.eq_ignore_ascii_case("VVS"));
    let vvs_price_usd = match vvs_token {
        Some(t) => infra::price::get_price_usd(services, t).await?,
        None => None,
    };

    for pool in pools {
        let balance_bytes = results_iter
            .next()
            .ok_or_else(|| CroLensError::RpcError("Missing multicall result".to_string()))?;
        let reserves_bytes = results_iter
            .next()
            .ok_or_else(|| CroLensError::RpcError("Missing multicall result".to_string()))?;
        let supply_bytes = results_iter
            .next()
            .ok_or_else(|| CroLensError::RpcError("Missing multicall result".to_string()))?;
        let staked_bytes =
            if pool.pool_index.is_some() {
                Some(results_iter.next().ok_or_else(|| {
                    CroLensError::RpcError("Missing multicall result".to_string())
                })?)
            } else {
                None
            };
        let pending_bytes =
            if pool.pool_index.is_some() {
                Some(results_iter.next().ok_or_else(|| {
                    CroLensError::RpcError("Missing multicall result".to_string())
                })?)
            } else {
                None
            };

        let Ok(balance_data) = balance_bytes else {
            continue;
        };
        let Ok(reserves_data) = reserves_bytes else {
            continue;
        };
        let Ok(supply_data) = supply_bytes else {
            continue;
        };

        let balance_ret = abi::balanceOfCall::abi_decode_returns(&balance_data, true)
            .map_err(|err| CroLensError::RpcError(format!("balanceOf decode failed: {err}")))?;
        let wallet_lp: U256 = balance_ret._0;

        let staked_lp: U256 = match staked_bytes {
            Some(Ok(data)) => {
                let decoded =
                    abi::userInfoCall::abi_decode_returns(&data, true).map_err(|err| {
                        CroLensError::RpcError(format!("userInfo decode failed: {err}"))
                    })?;
                decoded.amount
            }
            _ => U256::ZERO,
        };
        let user_lp = wallet_lp.saturating_add(staked_lp);
        if user_lp == U256::ZERO {
            continue;
        }

        let reserves_ret = abi::getReservesCall::abi_decode_returns(&reserves_data, true)
            .map_err(|err| CroLensError::RpcError(format!("getReserves decode failed: {err}")))?;
        let total_supply_ret = abi::totalSupplyCall::abi_decode_returns(&supply_data, true)
            .map_err(|err| CroLensError::RpcError(format!("totalSupply decode failed: {err}")))?;

        let total_supply: U256 = total_supply_ret._0;
        if total_supply == U256::ZERO {
            continue;
        }

        let reserve0 = U256::from(reserves_ret.reserve0);
        let reserve1 = U256::from(reserves_ret.reserve1);
        let token0_amount = reserve0.saturating_mul(user_lp) / total_supply;
        let token1_amount = reserve1.saturating_mul(user_lp) / total_supply;

        let pending_vvs = match pending_bytes {
            Some(Ok(data)) => {
                let decoded =
                    abi::pendingVVSCall::abi_decode_returns(&data, true).map_err(|err| {
                        CroLensError::RpcError(format!("pendingVVS decode failed: {err}"))
                    })?;
                decoded._0
            }
            _ => U256::ZERO,
        };
        let pending_vvs_formatted = types::format_units(&pending_vvs, 18);

        let token0 = token_map
            .iter()
            .find(|t| t.address == pool.token0_address)
            .cloned();
        let token1 = token_map
            .iter()
            .find(|t| t.address == pool.token1_address)
            .cloned();

        let token0_decimals = token0.as_ref().map(|t| t.decimals).unwrap_or(18);
        let token1_decimals = token1.as_ref().map(|t| t.decimals).unwrap_or(18);

        let token0_formatted = types::format_units(&token0_amount, token0_decimals);
        let token1_formatted = types::format_units(&token1_amount, token1_decimals);

        let token0_price = match token0.as_ref() {
            Some(t) => infra::price::get_price_usd(services, t).await?,
            None => None,
        };
        let token1_price = match token1.as_ref() {
            Some(t) => infra::price::get_price_usd(services, t).await?,
            None => None,
        };

        let value_usd = match (
            token0_price,
            token1_price,
            token0_formatted.parse::<f64>().ok(),
            token1_formatted.parse::<f64>().ok(),
        ) {
            (Some(p0), Some(p1), Some(a0), Some(a1)) => Some(p0 * a0 + p1 * a1),
            _ => None,
        };
        if let Some(v) = value_usd {
            vvs_total_liquidity_usd += v;
        }

        let pending_rewards_usd = match (vvs_price_usd, pending_vvs_formatted.parse::<f64>().ok()) {
            (Some(price), Some(amount)) => Some(price * amount),
            _ => None,
        };
        if let Some(v) = pending_rewards_usd {
            vvs_total_pending_rewards_usd += v;
        }
        vvs_total_pending_vvs = vvs_total_pending_vvs.saturating_add(pending_vvs);

        vvs_positions.push(serde_json::json!({
            "pool_id": pool.pool_id,
            "pool_name": format!("{}-{}", pool.token0_symbol, pool.token1_symbol),
            "lp_amount": user_lp.to_string(),
            "lp_wallet_amount": wallet_lp.to_string(),
            "lp_staked_amount": staked_lp.to_string(),
            "token0": {
                "address": pool.token0_address.to_string(),
                "symbol": pool.token0_symbol,
                "amount": token0_amount.to_string(),
                "amount_formatted": token0_formatted,
            },
            "token1": {
                "address": pool.token1_address.to_string(),
                "symbol": pool.token1_symbol,
                "amount": token1_amount.to_string(),
                "amount_formatted": token1_formatted,
            },
            "liquidity_usd": value_usd.map(|v| format!("{v:.2}")),
            "pending_rewards": { "vvs": pending_vvs_formatted.clone() },
            "pending_vvs": pending_vvs.to_string(),
            "pending_vvs_formatted": pending_vvs_formatted,
            "pending_rewards_usd": pending_rewards_usd.map(|v| format!("{v:.2}")),
            "apy": Value::Null,
        }));
    }

    let markets = infra::config::list_lending_markets(&services.db, "tectonic").await?;

    let mut supplies: Vec<Value> = Vec::new();
    let mut borrows: Vec<Value> = Vec::new();
    let mut total_supply_usd = 0.0_f64;
    let mut total_borrow_usd = 0.0_f64;
    let mut first_supply_detail: Option<String> = None;
    let mut first_borrow_detail: Option<String> = None;

    let mut market_calls = Vec::with_capacity(markets.len() * 3);
    for market in &markets {
        market_calls.push(infra::multicall::Call {
            target: market.ctoken_address,
            call_data: abi::getAccountSnapshotCall { account: user }
                .abi_encode()
                .into(),
        });
        market_calls.push(infra::multicall::Call {
            target: market.ctoken_address,
            call_data: abi::supplyRatePerBlockCall {}.abi_encode().into(),
        });
        market_calls.push(infra::multicall::Call {
            target: market.ctoken_address,
            call_data: abi::borrowRatePerBlockCall {}.abi_encode().into(),
        });
    }
    let market_results = services.multicall()?.aggregate(market_calls).await?;
    let mut market_iter = market_results.into_iter();

    for market in markets.into_iter() {
        let snapshot = market_iter
            .next()
            .ok_or_else(|| CroLensError::RpcError("Missing multicall result".to_string()))?;
        let supply_rate = market_iter
            .next()
            .ok_or_else(|| CroLensError::RpcError("Missing multicall result".to_string()))?;
        let borrow_rate = market_iter
            .next()
            .ok_or_else(|| CroLensError::RpcError("Missing multicall result".to_string()))?;

        let Ok(data) = snapshot else {
            continue;
        };

        let decoded =
            abi::getAccountSnapshotCall::abi_decode_returns(&data, true).map_err(|err| {
                CroLensError::RpcError(format!("getAccountSnapshot decode failed: {err}"))
            })?;
        if decoded.err != U256::ZERO {
            continue;
        }

        let supply_rate_per_block = match supply_rate {
            Ok(data) => {
                let decoded = abi::supplyRatePerBlockCall::abi_decode_returns(&data, true)
                    .map_err(|err| {
                        CroLensError::RpcError(format!("supplyRatePerBlock decode failed: {err}"))
                    })?;
                decoded._0
            }
            Err(_) => U256::ZERO,
        };
        let borrow_rate_per_block = match borrow_rate {
            Ok(data) => {
                let decoded = abi::borrowRatePerBlockCall::abi_decode_returns(&data, true)
                    .map_err(|err| {
                        CroLensError::RpcError(format!("borrowRatePerBlock decode failed: {err}"))
                    })?;
                decoded._0
            }
            Err(_) => U256::ZERO,
        };

        let supply_apy = apy_percent_string(supply_rate_per_block);
        let borrow_apy = apy_percent_string(borrow_rate_per_block);

        let token = token_map
            .iter()
            .find(|t| t.address == market.underlying_address)
            .cloned();
        let decimals = token.as_ref().map(|t| t.decimals).unwrap_or(18);
        let price = match token.as_ref() {
            Some(t) => infra::price::get_price_usd(services, t).await?,
            None => None,
        };

        let supply_underlying = decoded
            .cTokenBalance
            .saturating_mul(decoded.exchangeRateMantissa)
            / U256::from(1_000_000_000_000_000_000u128);
        let supply_formatted = types::format_units(&supply_underlying, decimals);
        let supply_value_usd = match (price, supply_formatted.parse::<f64>().ok()) {
            (Some(p), Some(a)) => Some(p * a),
            _ => None,
        };
        if let Some(v) = supply_value_usd {
            total_supply_usd += v;
        }

        let borrow_underlying = decoded.borrowBalance;
        let borrow_formatted = types::format_units(&borrow_underlying, decimals);
        let borrow_value_usd = match (price, borrow_formatted.parse::<f64>().ok()) {
            (Some(p), Some(a)) => Some(p * a),
            _ => None,
        };
        if let Some(v) = borrow_value_usd {
            total_borrow_usd += v;
        }

        if supply_underlying != U256::ZERO {
            if first_supply_detail.is_none() {
                if let (Some(v), Some(apy)) = (supply_value_usd, supply_apy.as_ref()) {
                    first_supply_detail = Some(format!(
                        "Supply {} ${v:.2} @{}",
                        market.underlying_symbol, apy
                    ));
                }
            }
            supplies.push(serde_json::json!({
                "market_address": market.ctoken_address.to_string(),
                "asset_symbol": market.underlying_symbol,
                "supply_balance": supply_underlying.to_string(),
                "supply_balance_usd": supply_value_usd.map(|v| format!("{v:.2}")),
                "supply_apy": supply_apy,
                "is_collateral": market.collateral_factor.is_some(),
            }));
        }

        if borrow_underlying != U256::ZERO {
            if first_borrow_detail.is_none() {
                if let (Some(v), Some(apy)) = (borrow_value_usd, borrow_apy.as_ref()) {
                    first_borrow_detail = Some(format!(
                        "Borrow {} ${v:.2} @{}",
                        market.underlying_symbol, apy
                    ));
                }
            }
            borrows.push(serde_json::json!({
                "market_address": market.ctoken_address.to_string(),
                "asset_symbol": market.underlying_symbol,
                "borrow_balance": borrow_underlying.to_string(),
                "borrow_balance_usd": borrow_value_usd.map(|v| format!("{v:.2}")),
                "borrow_apy": borrow_apy,
            }));
        }
    }

    let health_factor = health_factor_string(total_supply_usd, total_borrow_usd);

    if input.simple_mode {
        let pending_vvs_total_formatted = types::format_units(&vvs_total_pending_vvs, 18);
        let mut tectonic_details = Vec::new();
        if let Some(v) = first_supply_detail {
            tectonic_details.push(v);
        }
        if let Some(v) = first_borrow_detail {
            tectonic_details.push(v);
        }
        let tectonic_suffix = if tectonic_details.is_empty() {
            String::new()
        } else {
            format!(" ({})", tectonic_details.join(", "))
        };
        let summary = format!(
            "VVS: {} position(s), Pending {} VVS (${:.2}) | Tectonic: Supply ${:.2}, Borrow ${:.2}, Health {}{}",
            vvs_positions.len(),
            pending_vvs_total_formatted,
            vvs_total_pending_rewards_usd,
            total_supply_usd,
            total_borrow_usd,
            health_factor,
            tectonic_suffix
        );
        return Ok(serde_json::json!({ "text": summary, "meta": services.meta() }));
    }

    let net_value_usd = total_supply_usd - total_borrow_usd;

    Ok(serde_json::json!({
        "address": input.address,
        "vvs": {
            "total_liquidity_usd": format!("{vvs_total_liquidity_usd:.2}"),
            "total_pending_rewards_usd": format!("{vvs_total_pending_rewards_usd:.2}"),
            "positions": vvs_positions,
        },
        "tectonic": {
            "total_supply_usd": format!("{total_supply_usd:.2}"),
            "total_borrow_usd": format!("{total_borrow_usd:.2}"),
            "net_value_usd": format!("{net_value_usd:.2}"),
            "supplies": supplies,
            "borrows": borrows,
            "health_factor": health_factor,
        },
        "meta": services.meta(),
    }))
}

fn apy_percent_string(rate_per_block: U256) -> Option<String> {
    if rate_per_block == U256::ZERO {
        return Some("0.00%".to_string());
    }
    let rate = rate_per_block.to_string().parse::<f64>().ok()? / 1e18_f64;
    if !rate.is_finite() || rate <= 0.0 {
        return Some("0.00%".to_string());
    }

    let apy = (BLOCKS_PER_YEAR * rate.ln_1p()).exp_m1();
    if !apy.is_finite() || apy < 0.0 {
        return None;
    }

    Some(format!("{:.2}%", apy * 100.0))
}

fn health_factor_string(total_supply_usd: f64, total_borrow_usd: f64) -> String {
    if total_borrow_usd <= 0.0 {
        return "∞".to_string();
    }
    format!("{:.2}", total_supply_usd / total_borrow_usd)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_percent(value: &str) -> f64 {
        value.trim_end_matches('%').parse::<f64>().unwrap_or(0.0)
    }

    #[test]
    fn apy_zero_is_zero() {
        assert_eq!(apy_percent_string(U256::ZERO), Some("0.00%".to_string()));
    }

    #[test]
    fn apy_small_rate_is_non_negative() {
        let rate = U256::from(10_000_000_000u64);
        let value = apy_percent_string(rate).expect("apy must be present");
        assert!(value.ends_with('%'));
        assert!(parse_percent(&value) >= 0.0);
    }

    #[test]
    fn apy_increases_with_rate() {
        let low = U256::from(1_000_000_000u64);
        let high = U256::from(10_000_000_000u64);
        let low_apy = parse_percent(&apy_percent_string(low).unwrap());
        let high_apy = parse_percent(&apy_percent_string(high).unwrap());
        assert!(high_apy > low_apy);
    }

    #[test]
    fn apy_extreme_rate_returns_none() {
        let too_high = U256::from(1_000_000_000_000_000_000u128);
        assert!(apy_percent_string(too_high).is_none());
    }

    #[test]
    fn apy_tiny_rate_rounds_down() {
        let tiny = U256::from(1u64);
        let value = apy_percent_string(tiny).unwrap();
        assert_eq!(value, "0.00%");
    }

    #[test]
    fn apy_reasonable_rate_is_in_expected_range() {
        let rate = U256::from(10_000_000_000u64);
        let value = parse_percent(&apy_percent_string(rate).unwrap());
        assert!(value > 100.0);
        assert!(value < 1_000_000.0);
    }

    #[test]
    fn apy_very_large_rate_is_none() {
        let rate = U256::from(100_000_000_000_000_000u128);
        assert!(apy_percent_string(rate).is_none());
    }

    #[test]
    fn apy_does_not_panic_on_huge_u256() {
        let _ = apy_percent_string(U256::MAX);
    }

    #[test]
    fn health_factor_rounds_down() {
        assert_eq!(health_factor_string(1.0, 7.0), "0.14");
    }

    #[test]
    fn health_factor_rounds_up() {
        assert_eq!(health_factor_string(2.0, 3.0), "0.67");
    }

    #[test]
    fn health_factor_large_values() {
        assert_eq!(health_factor_string(1_000_000.0, 1.0), "1000000.00");
    }

    #[test]
    fn health_factor_borrow_zero_is_infinite_even_with_zero_supply() {
        assert_eq!(health_factor_string(0.0, 0.0), "∞");
    }

    #[test]
    fn health_factor_infinite_when_no_borrow() {
        assert_eq!(health_factor_string(1000.0, 0.0), "∞");
        assert_eq!(health_factor_string(1000.0, -1.0), "∞");
    }

    #[test]
    fn health_factor_formats_with_two_decimals() {
        assert_eq!(health_factor_string(1850.0, 1000.0), "1.85");
        assert_eq!(health_factor_string(1.0, 3.0), "0.33");
    }

    #[test]
    fn health_factor_handles_zero_supply() {
        assert_eq!(health_factor_string(0.0, 100.0), "0.00");
    }
}
