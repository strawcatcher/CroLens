use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;

#[derive(Debug, Deserialize)]
struct SimpleModeArgs {
    #[serde(default)]
    simple_mode: bool,
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
        let text = match price_usd {
            Some(p) => format!("CRO price: ${p:.4}"),
            None => "CRO overview available.".to_string(),
        };
        return Ok(serde_json::json!({ "text": text, "meta": services.meta() }));
    }

    Ok(serde_json::json!({
        "chain_id": 25,
        "block_number": block_number,
        "price_usd": price_usd.map(|p| format!("{p:.6}")),
        "meta": services.meta(),
    }))
}

