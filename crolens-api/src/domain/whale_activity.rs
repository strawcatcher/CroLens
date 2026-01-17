use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;

#[derive(Debug, Deserialize)]
struct WhaleActivityArgs {
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    min_value_usd: Option<f64>,
    #[serde(default)]
    blocks: Option<u64>,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_whale_activity(services: &infra::Services, args: Value) -> Result<Value> {
    let input: WhaleActivityArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": "Whale activity monitoring is not available in this build (placeholder).",
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "token": input.token,
        "min_value_usd": input.min_value_usd,
        "blocks": input.blocks,
        "events": [],
        "meta": services.meta(),
    }))
}

