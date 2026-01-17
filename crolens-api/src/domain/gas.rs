use alloy_primitives::U256;
use serde::Deserialize;
use serde_json::Value;

use crate::error::Result;
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct GetGasPriceArgs {
    #[serde(default)]
    simple_mode: bool,
}

/// Typical gas costs for common operations
const GAS_TRANSFER: u64 = 21_000;
const GAS_ERC20_TRANSFER: u64 = 65_000;
const GAS_APPROVE: u64 = 46_000;
const GAS_SWAP: u64 = 150_000;
const GAS_ADD_LIQUIDITY: u64 = 200_000;
const GAS_REMOVE_LIQUIDITY: u64 = 180_000;

fn gas_price_level(gas_price_gwei: f64) -> &'static str {
    if gas_price_gwei < 3000.0 {
        "low"
    } else if gas_price_gwei < 8000.0 {
        "medium"
    } else {
        "high"
    }
}

fn recommendation_for_level(level: &str) -> &'static str {
    match level {
        "low" => "Gas prices are low. Good time for large transactions.",
        "medium" => "Gas prices are moderate. Normal operations recommended.",
        "high" => "Gas prices are high. Consider waiting for lower fees.",
        _ => "Unable to determine gas level.",
    }
}

fn estimate_cost(gas_price: U256, gas: u64, cro_price_usd: f64) -> (String, String, String) {
    let cost_wei = gas_price * U256::from(gas);
    let cost_cro = types::format_units(&cost_wei, 18);
    let cost_cro_f64: f64 = cost_cro.parse().unwrap_or(0.0);
    let cost_usd = cost_cro_f64 * cro_price_usd;
    (
        gas.to_string(),
        format!("{:.6}", cost_cro_f64),
        format!("{:.4}", cost_usd),
    )
}

/// Get current gas price and estimated costs
pub async fn get_gas_price(services: &infra::Services, args: Value) -> Result<Value> {
    let input: GetGasPriceArgs = serde_json::from_value(args).unwrap_or(GetGasPriceArgs {
        simple_mode: false,
    });

    let rpc = services.rpc()?;

    // Fetch current gas price.
    let gas_price = rpc.eth_gas_price().await?;
    let gas_price_gwei = types::format_units(&gas_price, 9);
    let gas_price_f64: f64 = gas_price_gwei.parse().unwrap_or(0.0);

    // Try EIP-1559 fees (best-effort).
    let (base_fee, priority_fee) = get_eip1559_fees(rpc).await.unwrap_or((None, None));

    // Classify gas level.
    let level = gas_price_level(gas_price_f64);

    // Fetch CRO price for USD estimates.
    let cro_price_usd = get_cro_price(services).await.unwrap_or(0.1);

    // Estimate typical transaction costs.
    let (transfer_gas, transfer_cro, transfer_usd) = estimate_cost(gas_price, GAS_TRANSFER, cro_price_usd);
    let (erc20_gas, erc20_cro, erc20_usd) =
        estimate_cost(gas_price, GAS_ERC20_TRANSFER, cro_price_usd);
    let (approve_gas, approve_cro, approve_usd) = estimate_cost(gas_price, GAS_APPROVE, cro_price_usd);
    let (swap_gas, swap_cro, swap_usd) = estimate_cost(gas_price, GAS_SWAP, cro_price_usd);
    let (add_liq_gas, add_liq_cro, add_liq_usd) =
        estimate_cost(gas_price, GAS_ADD_LIQUIDITY, cro_price_usd);
    let (remove_liq_gas, remove_liq_cro, remove_liq_usd) =
        estimate_cost(gas_price, GAS_REMOVE_LIQUIDITY, cro_price_usd);

    let recommendation = recommendation_for_level(level);

    if input.simple_mode {
        let text = format!(
            "Gas: {:.0} gwei ({}) | Transfer: ~{} CRO (~${}) | Swap: ~{} CRO (~${})",
            gas_price_f64, level, transfer_cro, transfer_usd, swap_cro, swap_usd
        );
        return Ok(serde_json::json!({ "text": text }));
    }

    Ok(serde_json::json!({
        "current_gwei": format!("{:.2}", gas_price_f64),
        "level": level,
        "base_fee_gwei": base_fee.map(|v| format!("{:.2}", v)),
        "priority_fee_gwei": priority_fee.map(|v| format!("{:.2}", v)),
        "cro_price_usd": format!("{:.4}", cro_price_usd),
        "estimated_costs": {
            "cro_transfer": {
                "gas": transfer_gas,
                "cost_cro": transfer_cro,
                "cost_usd": transfer_usd
            },
            "erc20_transfer": {
                "gas": erc20_gas,
                "cost_cro": erc20_cro,
                "cost_usd": erc20_usd
            },
            "approve": {
                "gas": approve_gas,
                "cost_cro": approve_cro,
                "cost_usd": approve_usd
            },
            "swap": {
                "gas": swap_gas,
                "cost_cro": swap_cro,
                "cost_usd": swap_usd
            },
            "add_liquidity": {
                "gas": add_liq_gas,
                "cost_cro": add_liq_cro,
                "cost_usd": add_liq_usd
            },
            "remove_liquidity": {
                "gas": remove_liq_gas,
                "cost_cro": remove_liq_cro,
                "cost_usd": remove_liq_usd
            }
        },
        "recommendation": recommendation,
        "meta": services.meta()
    }))
}

/// Best-effort EIP-1559 fee hints.
async fn get_eip1559_fees(
    rpc: &infra::rpc::RpcClient,
) -> Result<(Option<f64>, Option<f64>)> {
    // Cronos may not fully support EIP-1559.
    let priority_fee = rpc.eth_max_priority_fee_per_gas().await.ok();
    let priority_gwei = priority_fee.map(|v| {
        let s = types::format_units(&v, 9);
        s.parse::<f64>().unwrap_or(0.0)
    });

    // Base fee is typically fetched from the latest block; omitted here.
    Ok((None, priority_gwei))
}

/// Resolve CRO price (best-effort).
async fn get_cro_price(services: &infra::Services) -> Result<f64> {
    // Try KV cache first.
    let key = "price:anchor:cro";
    if let Ok(Some(text)) = services.kv.get(key).text().await {
        if let Ok(price) = text.parse::<f64>() {
            return Ok(price);
        }
    }

    // Fallback.
    Ok(0.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_price_level_boundaries() {
        assert_eq!(gas_price_level(0.0), "low");
        assert_eq!(gas_price_level(2999.99), "low");
        assert_eq!(gas_price_level(3000.0), "medium");
        assert_eq!(gas_price_level(7999.99), "medium");
        assert_eq!(gas_price_level(8000.0), "high");
    }

    #[test]
    fn recommendation_messages() {
        assert!(recommendation_for_level("low").contains("low"));
        assert!(recommendation_for_level("medium").contains("moderate"));
        assert!(recommendation_for_level("high").contains("high"));
        assert_eq!(recommendation_for_level("unknown"), "Unable to determine gas level.");
    }

    #[test]
    fn estimate_cost_formats() {
        // 1 gwei and a transfer should cost 0.000021 CRO.
        let gas_price = U256::from(1_000_000_000u64);
        let (gas, cro, usd) = estimate_cost(gas_price, GAS_TRANSFER, 0.1);
        assert_eq!(gas, "21000");
        assert_eq!(cro, "0.000021");
        assert_eq!(usd, "0.0000");
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({});
        let args: GetGasPriceArgs = serde_json::from_value(json).expect("args should parse");
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({ "simple_mode": true });
        let args: GetGasPriceArgs = serde_json::from_value(json).expect("args should parse");
        assert!(args.simple_mode);
    }
}
