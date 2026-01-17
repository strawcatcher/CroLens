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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_deserialize_all_defaults() {
        let json = serde_json::json!({});
        let args: WhaleActivityArgs = serde_json::from_value(json).expect("should parse");
        assert!(args.token.is_none());
        assert!(args.min_value_usd.is_none());
        assert!(args.blocks.is_none());
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_with_token() {
        let json = serde_json::json!({ "token": "CRO" });
        let args: WhaleActivityArgs = serde_json::from_value(json).expect("should parse");
        assert_eq!(args.token, Some("CRO".to_string()));
    }

    #[test]
    fn args_deserialize_with_min_value() {
        let json = serde_json::json!({ "min_value_usd": 100000.0 });
        let args: WhaleActivityArgs = serde_json::from_value(json).expect("should parse");
        assert_eq!(args.min_value_usd, Some(100000.0));
    }

    #[test]
    fn args_deserialize_with_blocks() {
        let json = serde_json::json!({ "blocks": 1000 });
        let args: WhaleActivityArgs = serde_json::from_value(json).expect("should parse");
        assert_eq!(args.blocks, Some(1000));
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({ "simple_mode": true });
        let args: WhaleActivityArgs = serde_json::from_value(json).expect("should parse");
        assert!(args.simple_mode);
    }
}
