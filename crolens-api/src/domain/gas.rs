use alloy_primitives::U256;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
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

/// Get current gas price and estimated costs
pub async fn get_gas_price(services: &infra::Services, args: Value) -> Result<Value> {
    let input: GetGasPriceArgs = serde_json::from_value(args).unwrap_or(GetGasPriceArgs {
        simple_mode: false,
    });

    let rpc = services.rpc()?;

    // 获取当前 gas price
    let gas_price = rpc.eth_gas_price().await?;
    let gas_price_gwei = types::format_units(&gas_price, 9);
    let gas_price_f64: f64 = gas_price_gwei.parse().unwrap_or(0.0);

    // 尝试获取 EIP-1559 费用 (如果支持)
    let (base_fee, priority_fee) = get_eip1559_fees(rpc).await.unwrap_or((None, None));

    // 判断 gas 水平
    let level = if gas_price_f64 < 3000.0 {
        "low"
    } else if gas_price_f64 < 8000.0 {
        "medium"
    } else {
        "high"
    };

    // 获取 CRO 价格用于计算成本
    let cro_price_usd = get_cro_price(services).await.unwrap_or(0.1);

    // 计算各种操作的估算成本
    let estimate_cost = |gas: u64| -> (String, String, String) {
        let cost_wei = gas_price * U256::from(gas);
        let cost_cro = types::format_units(&cost_wei, 18);
        let cost_cro_f64: f64 = cost_cro.parse().unwrap_or(0.0);
        let cost_usd = cost_cro_f64 * cro_price_usd;
        (
            gas.to_string(),
            format!("{:.6}", cost_cro_f64),
            format!("{:.4}", cost_usd),
        )
    };

    let (transfer_gas, transfer_cro, transfer_usd) = estimate_cost(GAS_TRANSFER);
    let (erc20_gas, erc20_cro, erc20_usd) = estimate_cost(GAS_ERC20_TRANSFER);
    let (approve_gas, approve_cro, approve_usd) = estimate_cost(GAS_APPROVE);
    let (swap_gas, swap_cro, swap_usd) = estimate_cost(GAS_SWAP);
    let (add_liq_gas, add_liq_cro, add_liq_usd) = estimate_cost(GAS_ADD_LIQUIDITY);
    let (remove_liq_gas, remove_liq_cro, remove_liq_usd) = estimate_cost(GAS_REMOVE_LIQUIDITY);

    // 生成建议
    let recommendation = match level {
        "low" => "Gas prices are low. Good time for large transactions.",
        "medium" => "Gas prices are moderate. Normal operations recommended.",
        "high" => "Gas prices are high. Consider waiting for lower fees.",
        _ => "Unable to determine gas level.",
    };

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

/// 尝试获取 EIP-1559 费用
async fn get_eip1559_fees(
    rpc: &infra::rpc::RpcClient,
) -> Result<(Option<f64>, Option<f64>)> {
    // Cronos 可能不完全支持 EIP-1559，尝试获取
    let priority_fee = rpc.eth_max_priority_fee_per_gas().await.ok();
    let priority_gwei = priority_fee.map(|v| {
        let s = types::format_units(&v, 9);
        s.parse::<f64>().unwrap_or(0.0)
    });

    // Base fee 通常从最新区块获取，这里简化处理
    Ok((None, priority_gwei))
}

/// 获取 CRO 价格
async fn get_cro_price(services: &infra::Services) -> Result<f64> {
    // 尝试从 KV 缓存获取 CRO 价格
    let key = "price:anchor:cro";
    if let Ok(Some(text)) = services.kv.get(key).text().await {
        if let Ok(price) = text.parse::<f64>() {
            return Ok(price);
        }
    }

    // 回退到默认值
    Ok(0.1)
}
