use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct PortfolioArgs {
    address: String,
    #[serde(default)]
    simple_mode: bool,
}

fn validate_address(address: &str) -> Result<()> {
    let _ = types::parse_address(address)?;
    Ok(())
}

pub async fn get_portfolio_analysis(services: &infra::Services, args: Value) -> Result<Value> {
    let input: PortfolioArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    validate_address(&input.address)?;

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": "Portfolio analysis is a placeholder in this build.",
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "address": input.address,
        "diversification_score": 0,
        "insights": [],
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
        let args: PortfolioArgs = serde_json::from_value(json).expect("should parse");
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({
            "address": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23",
            "simple_mode": true
        });
        let args: PortfolioArgs = serde_json::from_value(json).expect("should parse");
        assert!(args.simple_mode);
    }

    #[test]
    fn args_rejects_missing_address() {
        let json = serde_json::json!({});
        let result: std::result::Result<PortfolioArgs, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
