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

fn normalize_asset_filter(asset: &Option<String>) -> Option<String> {
    asset.as_ref().map(|s| s.trim().to_lowercase())
}

fn symbol_matches_asset_filter(symbol: &str, asset_filter: Option<&str>) -> bool {
    match asset_filter {
        Some(f) => symbol.trim().eq_ignore_ascii_case(f),
        None => true,
    }
}

pub async fn get_tectonic_rates(services: &infra::Services, args: Value) -> Result<Value> {
    let input: TectonicRatesArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let markets =
        infra::config::list_lending_markets_cached(&services.db, &services.kv, "tectonic").await?;

    let asset_filter = normalize_asset_filter(&input.asset);
    let out: Vec<Value> = markets
        .into_iter()
        .filter(|m| symbol_matches_asset_filter(&m.underlying_symbol, asset_filter.as_deref()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_asset_filter_trims_and_lowercases() {
        assert_eq!(normalize_asset_filter(&None), None);
        assert_eq!(
            normalize_asset_filter(&Some(" USDC ".to_string())),
            Some("usdc".to_string())
        );
        assert_eq!(
            normalize_asset_filter(&Some("".to_string())),
            Some("".to_string())
        );
    }

    #[test]
    fn symbol_matches_asset_filter_behaviour() {
        assert!(symbol_matches_asset_filter("USDC", None));
        assert!(symbol_matches_asset_filter("USDC", Some("usdc")));
        assert!(symbol_matches_asset_filter(" usdc ", Some("USDC")));
        assert!(!symbol_matches_asset_filter("CRO", Some("usdc")));
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({});
        let args: SimpleModeArgs = serde_json::from_value(json).expect("args should parse");
        assert!(!args.simple_mode);

        let json = serde_json::json!({});
        let args: TectonicRatesArgs = serde_json::from_value(json).expect("args should parse");
        assert!(args.asset.is_none());
        assert!(!args.simple_mode);
    }

    #[test]
    fn rates_args_deserialize_with_asset() {
        let json = serde_json::json!({ "asset": "USDC", "simple_mode": true });
        let args: TectonicRatesArgs = serde_json::from_value(json).expect("args should parse");
        assert_eq!(args.asset.as_deref(), Some("USDC"));
        assert!(args.simple_mode);
    }
}
