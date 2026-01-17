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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_known_transfer() {
        let data = "0xa9059cbb0000000000000000000000001234567890123456789012345678901234567890000000000000000000000000000000000000000000000000000000000000000a";
        let bytes = types::hex0x_to_bytes(data).expect("valid hex calldata");
        let (method, params) = decode_known("0xa9059cbb", &bytes);
        assert_eq!(method, "transfer");
        assert_eq!(
            params.get("to").and_then(|v| v.as_str()),
            Some("0x1234567890123456789012345678901234567890")
        );
        assert_eq!(params.get("amount").and_then(|v| v.as_str()), Some("10"));
    }

    #[test]
    fn decode_known_approve() {
        let data = "0x095ea7b3000000000000000000000000145863eb42cf62847a6ca784e6416c1682b1b2aeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let bytes = types::hex0x_to_bytes(data).expect("valid hex calldata");
        let (method, params) = decode_known("0x095ea7b3", &bytes);
        assert_eq!(method, "approve");
        assert_eq!(
            params.get("spender").and_then(|v| v.as_str()),
            Some("0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae")
        );
    }

    #[test]
    fn decode_known_unknown_selector() {
        let bytes = vec![0u8; 4];
        let (method, params) = decode_known("0xdeadbeef", &bytes);
        assert_eq!(method, "unknown");
        assert!(params.is_null());
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({ "data": "0xa9059cbb" });
        let args: DecodeCalldataArgs = serde_json::from_value(json).expect("args should parse");
        assert_eq!(args.data, "0xa9059cbb");
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({ "data": "0xa9059cbb", "simple_mode": true });
        let args: DecodeCalldataArgs = serde_json::from_value(json).expect("args should parse");
        assert!(args.simple_mode);
    }
}
