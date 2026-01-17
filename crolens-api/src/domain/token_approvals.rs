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

fn validate_address(address: &str) -> Result<()> {
    let _ = types::parse_address(address)?;
    Ok(())
}

pub async fn get_token_approvals(services: &infra::Services, args: Value) -> Result<Value> {
    let input: TokenApprovalsArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    validate_address(&input.address)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_address_rejects_invalid() {
        let err = validate_address("0x123").unwrap_err();
        assert!(matches!(err, CroLensError::InvalidAddress(_)));
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({ "address": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23" });
        let args: TokenApprovalsArgs = serde_json::from_value(json).expect("should parse");
        assert!(!args.include_zero);
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_include_zero() {
        let json = serde_json::json!({
            "address": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23",
            "include_zero": true
        });
        let args: TokenApprovalsArgs = serde_json::from_value(json).expect("should parse");
        assert!(args.include_zero);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({
            "address": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23",
            "simple_mode": true
        });
        let args: TokenApprovalsArgs = serde_json::from_value(json).expect("should parse");
        assert!(args.simple_mode);
    }

    #[test]
    fn args_rejects_missing_address() {
        let json = serde_json::json!({});
        let result: std::result::Result<TokenApprovalsArgs, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
