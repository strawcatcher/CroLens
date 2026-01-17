use alloy_sol_types::SolCall;
use serde::Deserialize;
use serde_json::Value;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct DecodeCalldataArgs {
    data: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn decode_calldata(services: &infra::Services, args: Value) -> Result<Value> {
    let input: DecodeCalldataArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let data = input.data.trim();
    if !data.starts_with("0x") {
        return Err(CroLensError::invalid_params(
            "data must be 0x-prefixed hex".to_string(),
        ));
    }

    let bytes = types::hex0x_to_bytes(data)?;
    let selector = if bytes.len() >= 4 {
        format!("0x{}", hex::encode(&bytes[..4]))
    } else {
        "0x".to_string()
    };

    let (method, params) = decode_known(&selector, &bytes);

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": format!("Calldata: {method}"),
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "selector": selector,
        "method": method,
        "params": params,
        "meta": services.meta(),
    }))
}

fn decode_known(selector: &str, bytes: &[u8]) -> (String, Value) {
    match selector {
        "0xa9059cbb" => {
            if let Ok(decoded) = abi::transferCall::abi_decode(bytes, true) {
                return (
                    "transfer".to_string(),
                    serde_json::json!({
                        "to": decoded.recipient.to_string(),
                        "amount": decoded.amount.to_string(),
                    }),
                );
            }
        }
        "0x095ea7b3" => {
            if let Ok(decoded) = abi::approveCall::abi_decode(bytes, true) {
                return (
                    "approve".to_string(),
                    serde_json::json!({
                        "spender": decoded.spender.to_string(),
                        "amount": decoded.amount.to_string(),
                    }),
                );
            }
        }
        "0x23b872dd" => {
            if let Ok(decoded) = abi::transferFromCall::abi_decode(bytes, true) {
                return (
                    "transferFrom".to_string(),
                    serde_json::json!({
                        "from": decoded.sender.to_string(),
                        "to": decoded.recipient.to_string(),
                        "amount": decoded.amount.to_string(),
                    }),
                );
            }
        }
        _ => {}
    }

    ("unknown".to_string(), Value::Null)
}

