use alloy_primitives::U256;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

fn default_hex_data() -> String {
    "0x".to_string()
}

fn default_value() -> String {
    "0".to_string()
}

#[derive(Debug, Deserialize)]
struct EstimateGasArgs {
    from: String,
    to: String,
    #[serde(default = "default_hex_data")]
    data: String,
    #[serde(default = "default_value")]
    value: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn estimate_gas(services: &infra::Services, args: Value) -> Result<Value> {
    let input: EstimateGasArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let from = types::parse_address(&input.from)?;
    let to = types::parse_address(&input.to)?;

    let data = input.data.trim();
    if !data.starts_with("0x") {
        return Err(CroLensError::invalid_params(
            "data must be 0x-prefixed hex".to_string(),
        ));
    }
    let _ = types::hex0x_to_bytes(data)?;

    let value_u256 = if input.value.trim().starts_with("0x") {
        types::parse_u256_hex(&input.value)?
    } else {
        types::parse_u256_dec(&input.value)?
    };

    let Ok(rpc) = services.rpc() else {
        return Ok(serde_json::json!({
            "text": "Gas estimation not available (RPC not configured).",
            "meta": services.meta(),
        }));
    };

    let tx_obj = serde_json::json!({
        "from": from.to_string(),
        "to": to.to_string(),
        "data": data,
        "value": format!("0x{:x}", value_u256),
    });

    let result = rpc
        .call("eth_estimateGas", serde_json::json!([tx_obj]))
        .await?;
    let gas_hex = result.as_str().ok_or_else(|| {
        CroLensError::RpcError("eth_estimateGas result is not a string".to_string())
    })?;
    let gas: U256 = types::parse_u256_hex(gas_hex)?;

    let gas_price_wei = rpc.eth_gas_price().await.ok();
    let (estimated_cost_wei, estimated_cost_cro) = match gas_price_wei {
        Some(price) => {
            let wei = gas.saturating_mul(price);
            let cro = types::format_units(&wei, 18);
            (Some(wei), Some(cro))
        }
        None => (None, None),
    };

    if input.simple_mode {
        let mut text = format!("Estimated gas: {}", gas.to_string());
        if let Some(cro) = &estimated_cost_cro {
            text.push_str(&format!(" | Estimated cost: {cro} CRO"));
        }
        return Ok(serde_json::json!({ "text": text, "meta": services.meta() }));
    }

    Ok(serde_json::json!({
        "from": input.from,
        "to": input.to,
        "gas_estimate": gas.to_string(),
        "gas_price_wei": gas_price_wei.map(|v| v.to_string()),
        "estimated_cost_wei": estimated_cost_wei.map(|v| v.to_string()),
        "estimated_cost_cro": estimated_cost_cro,
        "meta": services.meta(),
    }))
}

