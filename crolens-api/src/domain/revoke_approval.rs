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

