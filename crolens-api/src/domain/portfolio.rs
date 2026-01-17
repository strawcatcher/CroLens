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

pub async fn get_portfolio_analysis(services: &infra::Services, args: Value) -> Result<Value> {
    let input: PortfolioArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let _ = types::parse_address(&input.address)?;

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

