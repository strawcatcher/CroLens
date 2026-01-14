use alloy_primitives::U256;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct SimulateArgs {
    from: String,
    to: String,
    data: String,
    value: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn simulate_transaction(services: &infra::Services, args: Value) -> Result<Value> {
    let input: SimulateArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let from = types::parse_address(&input.from)?;
    let to = types::parse_address(&input.to)?;
    if !input.data.trim().starts_with("0x") {
        return Err(CroLensError::invalid_params(
            "data must be 0x-prefixed hex".to_string(),
        ));
    }
    let _data_bytes = types::hex0x_to_bytes(&input.data)?;

    let value = if input.value.trim().starts_with("0x") {
        types::parse_u256_hex(&input.value)?
    } else {
        types::parse_u256_dec(&input.value)?
    };

    let Some(tenderly) = services.tenderly() else {
        if input.simple_mode {
            return Ok(serde_json::json!({
                "text": "Simulation not available (Tenderly not configured).",
                "meta": services.meta(),
            }));
        }

        return Ok(serde_json::json!({
            "success": false,
            "simulation_available": false,
            "meta": services.meta(),
        }));
    };

    let simulation = tenderly.simulate(from, to, &input.data, value).await?;
    let gas_estimated = simulation
        .gas_used
        .map(|v| v.to_string())
        .unwrap_or_else(|| "0".to_string());

    let state_changes = decode_state_changes(&simulation.logs);
    let risk_level = if simulation.success { "low" } else { "high" };
    let warnings = simulation
        .error_message
        .as_ref()
        .map(|m| vec![m.clone()])
        .unwrap_or_default();

    if input.simple_mode {
        let text = if simulation.success {
            format!("Simulation success | Gas used: {gas_estimated}")
        } else {
            format!(
                "Simulation failed | Reason: {}",
                simulation
                    .error_message
                    .unwrap_or_else(|| "unknown".to_string())
            )
        };
        return Ok(serde_json::json!({
            "text": text,
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "success": simulation.success,
        "gas_estimated": gas_estimated,
        "estimated_cost_cro": Value::Null,
        "return_data": Value::Null,
        "decoded_return": Value::Null,
        "state_changes": state_changes,
        "risk_assessment": { "level": risk_level, "warnings": warnings },
        "meta": services.meta(),
    }))
}

fn decode_state_changes(logs: &[infra::tenderly::TenderlyLog]) -> Vec<Value> {
    const TRANSFER_TOPIC0: &str =
        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

    let mut out = Vec::new();
    for log in logs {
        if log.topics.len() >= 3 && log.topics[0].eq_ignore_ascii_case(TRANSFER_TOPIC0) {
            let from = topic_to_address(&log.topics[1]);
            let to = topic_to_address(&log.topics[2]);
            let amount = types::parse_u256_hex(&log.data).unwrap_or(U256::ZERO);

            out.push(serde_json::json!({
                "type": "transfer",
                "description": "ERC20 Transfer",
                "from": from,
                "to": to,
                "amount": amount.to_string(),
                "token": log.address,
            }));
        }
    }

    out
}

fn topic_to_address(topic: &str) -> String {
    let trimmed = topic.trim().trim_start_matches("0x");
    if trimmed.len() < 40 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    let addr_hex = &trimmed[trimmed.len() - 40..];
    format!("0x{addr_hex}")
}
