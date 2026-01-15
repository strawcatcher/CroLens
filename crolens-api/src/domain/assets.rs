use alloy_primitives::U256;
use alloy_sol_types::SolCall;
use serde::Deserialize;
use serde_json::Value;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct GetAccountSummaryArgs {
    address: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_account_summary(services: &infra::Services, args: Value) -> Result<Value> {
    let input: GetAccountSummaryArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;
    let address = types::parse_address(&input.address)?;

    let tokens = infra::token::list_tokens_cached(&services.db, &services.kv).await?;
    let mut calls = Vec::with_capacity(tokens.len());
    for token in &tokens {
        let call_data = abi::balanceOfCall { account: address }.abi_encode();
        calls.push(infra::multicall::Call {
            target: token.address,
            call_data: call_data.into(),
        });
    }

    let results = services.multicall()?.aggregate(calls).await?;

    // 批量获取所有代币价格（并行查询 KV）
    let price_map = infra::price::get_prices_usd_batch(services, &tokens).await?;

    let mut wallet = Vec::new();
    let mut wallet_value_usd = 0.0_f64;

    for (token, item) in tokens.into_iter().zip(results.into_iter()) {
        let Ok(return_data) = item else {
            continue;
        };
        let decoded = abi::balanceOfCall::abi_decode_returns(&return_data, true)
            .map_err(|err| CroLensError::RpcError(format!("balanceOf decode failed: {err}")))?;
        let balance: U256 = decoded._0;
        if balance == U256::ZERO {
            continue;
        }

        let balance_formatted = types::format_units(&balance, token.decimals);
        let price_usd = price_map.get(&token.address).copied();
        let value_usd = match (price_usd, balance_formatted.parse::<f64>().ok()) {
            (Some(p), Some(amount)) => {
                let v = p * amount;
                wallet_value_usd += v;
                Some(v)
            }
            _ => None,
        };

        wallet.push(serde_json::json!({
            "token_address": token.address.to_string(),
            "symbol": token.symbol,
            "decimals": token.decimals,
            "balance": balance.to_string(),
            "balance_formatted": balance_formatted,
            "price_usd": price_usd.map(|p| format!("{p:.6}")),
            "value_usd": value_usd.map(|v| format!("{v:.2}")),
        }));
    }

    if input.simple_mode {
        let summary = format!(
            "Wallet tokens: {} | Wallet value: ${wallet_value_usd:.2}",
            wallet.len(),
        );
        return Ok(serde_json::json!({ "text": summary, "meta": services.meta() }));
    }

    let mut vvs_liquidity_usd = 0.0_f64;
    let mut tectonic_supply_usd = 0.0_f64;
    let mut tectonic_borrow_usd = 0.0_f64;

    if let Ok(defi) = crate::domain::defi::get_defi_positions(
        services,
        serde_json::json!({ "address": input.address, "simple_mode": false }),
    )
    .await
    {
        vvs_liquidity_usd = defi
            .get("vvs")
            .and_then(|v| v.get("total_liquidity_usd"))
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        tectonic_supply_usd = defi
            .get("tectonic")
            .and_then(|v| v.get("total_supply_usd"))
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        tectonic_borrow_usd = defi
            .get("tectonic")
            .and_then(|v| v.get("total_borrow_usd"))
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
    }

    let total_defi_value_usd = vvs_liquidity_usd + (tectonic_supply_usd - tectonic_borrow_usd);
    let total_net_worth_usd = wallet_value_usd + total_defi_value_usd;

    Ok(serde_json::json!({
        "address": input.address,
        "total_net_worth_usd": format!("{total_net_worth_usd:.2}"),
        "wallet": wallet,
        "defi_summary": {
            "total_defi_value_usd": format!("{total_defi_value_usd:.2}"),
            "vvs_liquidity_usd": format!("{vvs_liquidity_usd:.2}"),
            "tectonic_supply_usd": format!("{tectonic_supply_usd:.2}"),
            "tectonic_borrow_usd": format!("{tectonic_borrow_usd:.2}"),
        },
        "meta": services.meta(),
    }))
}
