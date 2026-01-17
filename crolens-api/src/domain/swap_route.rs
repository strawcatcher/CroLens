use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;

#[derive(Debug, Deserialize)]
struct BestSwapRouteArgs {
    token_in: String,
    token_out: String,
    amount_in: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_best_swap_route(services: &infra::Services, args: Value) -> Result<Value> {
    let input: BestSwapRouteArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    // Currently, VVS is the only supported DEX in this repo.
    let route = serde_json::json!({
        "dex": "vvs",
        "path": [input.token_in, input.token_out],
        "estimated_out": Value::Null,
    });

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": "Best swap route: vvs (placeholder).",
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "amount_in": input.amount_in,
        "best_route": route,
        "routes": [route],
        "meta": services.meta(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({
            "token_in": "CRO",
            "token_out": "USDC",
            "amount_in": "1000000000000000000"
        });
        let args: BestSwapRouteArgs = serde_json::from_value(json).expect("should parse");
        assert_eq!(args.token_in, "CRO");
        assert_eq!(args.token_out, "USDC");
        assert_eq!(args.amount_in, "1000000000000000000");
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({
            "token_in": "CRO",
            "token_out": "USDC",
            "amount_in": "1000000000000000000",
            "simple_mode": true
        });
        let args: BestSwapRouteArgs = serde_json::from_value(json).expect("should parse");
        assert!(args.simple_mode);
    }

    #[test]
    fn args_rejects_missing_token_in() {
        let json = serde_json::json!({
            "token_out": "USDC",
            "amount_in": "1000000000000000000"
        });
        let result: std::result::Result<BestSwapRouteArgs, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn args_rejects_missing_amount_in() {
        let json = serde_json::json!({
            "token_in": "CRO",
            "token_out": "USDC"
        });
        let result: std::result::Result<BestSwapRouteArgs, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
