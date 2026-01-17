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

