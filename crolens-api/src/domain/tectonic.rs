use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;

#[derive(Debug, Deserialize)]
struct SimpleModeArgs {
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_tectonic_markets(services: &infra::Services, args: Value) -> Result<Value> {
    let input: SimpleModeArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let markets =
        infra::config::list_lending_markets_cached(&services.db, &services.kv, "tectonic").await?;
    let out: Vec<Value> = markets
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "ctoken_address": m.ctoken_address.to_string(),
                "underlying_address": m.underlying_address.to_string(),
                "underlying_symbol": m.underlying_symbol,
                "collateral_factor": m.collateral_factor,
            })
        })
        .collect();

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": format!("Tectonic markets: {}", out.len()),
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({ "markets": out, "meta": services.meta() }))
}

#[derive(Debug, Deserialize)]
struct TectonicRatesArgs {
    #[serde(default)]
    asset: Option<String>,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_tectonic_rates(services: &infra::Services, args: Value) -> Result<Value> {
    let input: TectonicRatesArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let markets =
        infra::config::list_lending_markets_cached(&services.db, &services.kv, "tectonic").await?;

    let asset_filter = input.asset.as_ref().map(|s| s.trim().to_lowercase());
    let out: Vec<Value> = markets
        .into_iter()
        .filter(|m| {
            if let Some(filter) = &asset_filter {
                m.underlying_symbol.trim().to_lowercase() == *filter
            } else {
                true
            }
        })
        .map(|m| {
            serde_json::json!({
                "underlying_symbol": m.underlying_symbol,
                "supply_apy": Value::Null,
                "borrow_apy": Value::Null,
            })
        })
        .collect();

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": format!("Tectonic rates: {}", out.len()),
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "asset": input.asset,
        "rates": out,
        "meta": services.meta(),
    }))
}

