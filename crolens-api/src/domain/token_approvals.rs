use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct TokenApprovalsArgs {
    address: String,
    #[serde(default)]
    include_zero: bool,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_token_approvals(services: &infra::Services, args: Value) -> Result<Value> {
    let input: TokenApprovalsArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let _ = types::parse_address(&input.address)?;

    let mut approvals: Vec<Value> = Vec::new();
    if let Ok(value) = crate::domain::approval::get_approval_status(
        services,
        serde_json::json!({ "address": input.address, "simple_mode": false }),
    )
    .await
    {
        approvals = value
            .get("approvals")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
    }

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": format!("Token approvals: {} (include_zero={} - placeholder)", approvals.len(), input.include_zero),
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "address": input.address,
        "include_zero": input.include_zero,
        "approvals": approvals,
        "meta": services.meta(),
    }))
}

