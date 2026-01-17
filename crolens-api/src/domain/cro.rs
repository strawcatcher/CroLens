use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;

#[derive(Debug, Deserialize)]
struct SimpleModeArgs {
    #[serde(default)]
    simple_mode: bool,
}

const CRO_CHAIN_ID: u64 = 25;

fn format_cro_price_text(price_usd: Option<f64>) -> String {
    match price_usd {
        Some(p) => format!("CRO price: ${p:.4}"),
        None => "CRO overview available.".to_string(),
    }
}

fn format_price_usd(price_usd: Option<f64>) -> Option<String> {
    price_usd.map(|p| format!("{p:.6}"))
}

pub async fn get_cro_overview(services: &infra::Services, args: Value) -> Result<Value> {
    let input: SimpleModeArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let mut block_number: Option<String> = None;
    if let Ok(rpc) = services.rpc() {
        if let Ok(result) = rpc.call("eth_blockNumber", serde_json::json!([])).await {
            block_number = result.as_str().map(|v| v.to_string());
        }
    }

    let mut price_usd: Option<f64> = None;
    if let Ok(tokens) = infra::token::list_tokens_cached(&services.db, &services.kv).await {
        if let Some(wcro) = tokens.iter().find(|t| t.symbol.eq_ignore_ascii_case("WCRO")) {
            price_usd = infra::price::get_price_usd(services, wcro).await.ok().flatten();
        }
    }

    if input.simple_mode {
        let text = format_cro_price_text(price_usd);
        return Ok(serde_json::json!({ "text": text, "meta": services.meta() }));
    }

    Ok(serde_json::json!({
        "chain_id": CRO_CHAIN_ID,
        "block_number": block_number,
        "price_usd": format_price_usd(price_usd),
        "meta": services.meta(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_cro_price_text_variants() {
        assert_eq!(format_cro_price_text(None), "CRO overview available.");
        assert_eq!(format_cro_price_text(Some(1.2345)), "CRO price: $1.2345");
    }

    #[test]
    fn format_price_usd_formats_six_decimals() {
        assert_eq!(format_price_usd(None), None);
        assert_eq!(format_price_usd(Some(1.0)), Some("1.000000".to_string()));
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({});
        let args: SimpleModeArgs = serde_json::from_value(json).expect("args should parse");
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({ "simple_mode": true });
        let args: SimpleModeArgs = serde_json::from_value(json).expect("args should parse");
        assert!(args.simple_mode);
    }
}
