use alloy_primitives::U256;
use alloy_sol_types::SolCall;
use serde::Deserialize;
use serde_json::Value;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct RevokeApprovalArgs {
    token: String,
    spender: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn construct_revoke_approval(services: &infra::Services, args: Value) -> Result<Value> {
    let input: RevokeApprovalArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let token_address = if input.token.trim().starts_with("0x") {
        types::parse_address(&input.token)?
    } else {
        let tokens = infra::token::list_tokens_cached(&services.db, &services.kv).await?;
        infra::token::resolve_token(&tokens, &input.token)?.address
    };
    let spender = types::parse_address(&input.spender)?;

    let calldata = abi::approveCall {
        spender,
        amount: U256::ZERO,
    }
    .abi_encode();

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": format!("Revoke approval calldata for token {}", token_address),
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "token_address": token_address.to_string(),
        "spender_address": spender.to_string(),
        "tx_data": {
            "to": token_address.to_string(),
            "data": types::bytes_to_hex0x(&calldata),
            "value": "0",
        },
        "meta": services.meta(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({
            "token": "VVS",
            "spender": "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae"
        });
        let args: RevokeApprovalArgs = serde_json::from_value(json).expect("args should parse");
        assert_eq!(args.token, "VVS");
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({
            "token": "0x2D03bece6747ADC00E1a131BBA1469C15fD11e03",
            "spender": "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae",
            "simple_mode": true
        });
        let args: RevokeApprovalArgs = serde_json::from_value(json).expect("args should parse");
        assert!(args.simple_mode);
    }

    #[test]
    fn revoke_approval_calldata_is_approve_zero() {
        let spender =
            types::parse_address("0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae").unwrap();
        let calldata = abi::approveCall {
            spender,
            amount: U256::ZERO,
        }
        .abi_encode();

        assert_eq!(calldata.len(), 4 + 32 + 32);
        let hex = types::bytes_to_hex0x(&calldata);
        assert!(hex.starts_with("0x095ea7b3"));
        assert!(hex.ends_with(&"0".repeat(64)));
    }
}
