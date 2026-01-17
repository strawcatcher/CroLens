use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

fn default_protocol() -> String {
    "tectonic".to_string()
}

fn classify_liquidation_risk(health_factor: Option<&str>) -> (&'static str, Option<&'static str>) {
    match health_factor {
        Some("∞") => ("low", None),
        Some(v) => match v.parse::<f64>() {
            Ok(hf) if hf < 1.1 => ("high", Some("Health factor is below 1.1")),
            Ok(hf) if hf < 1.5 => ("medium", Some("Health factor is below 1.5")),
            Ok(_) => ("low", None),
            Err(_) => ("unknown", Some("Unable to parse health factor")),
        },
        None => ("unknown", Some("Health factor unavailable")),
    }
}

#[derive(Debug, Deserialize)]
struct LendingRatesArgs {
    #[serde(default)]
    asset: Option<String>,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_lending_rates(services: &infra::Services, args: Value) -> Result<Value> {
    let input: LendingRatesArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    // Only Tectonic is supported today.
    let rates = vec![serde_json::json!({
        "protocol": "tectonic",
        "asset": input.asset,
        "supply_apy": Value::Null,
        "borrow_apy": Value::Null,
    })];

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": "Lending rates (tectonic only).",
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({ "rates": rates, "meta": services.meta() }))
}

#[derive(Debug, Deserialize)]
struct LiquidationRiskArgs {
    address: String,
    #[serde(default = "default_protocol")]
    protocol: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_liquidation_risk(services: &infra::Services, args: Value) -> Result<Value> {
    let input: LiquidationRiskArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let _ = types::parse_address(&input.address)?;

    let protocol = input.protocol.trim().to_lowercase();
    if protocol != "tectonic" {
        return Err(CroLensError::invalid_params(
            "Only protocol 'tectonic' is supported in this build".to_string(),
        ));
    }

    let mut health_factor: Option<String> = None;
    if let Ok(defi) = crate::domain::defi::get_defi_positions(
        services,
        serde_json::json!({ "address": input.address, "simple_mode": false }),
    )
    .await
    {
        health_factor = defi
            .get("tectonic")
            .and_then(|v| v.get("health_factor"))
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
    }

    let (risk_level, warning) = classify_liquidation_risk(health_factor.as_deref());

    if input.simple_mode {
        let hf_display = health_factor.clone().unwrap_or_else(|| "unknown".to_string());
        return Ok(serde_json::json!({
            "text": format!("Liquidation risk: {risk_level} | Health factor: {hf_display}"),
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "address": input.address,
        "protocol": protocol,
        "health_factor": health_factor,
        "risk_level": risk_level,
        "warning": warning,
        "meta": services.meta(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_protocol_is_tectonic() {
        assert_eq!(default_protocol(), "tectonic");
    }

    #[test]
    fn classify_liquidation_risk_variants() {
        assert_eq!(
            classify_liquidation_risk(None),
            ("unknown", Some("Health factor unavailable"))
        );
        assert_eq!(classify_liquidation_risk(Some("∞")), ("low", None));
        assert_eq!(
            classify_liquidation_risk(Some("1.09")),
            ("high", Some("Health factor is below 1.1"))
        );
        assert_eq!(
            classify_liquidation_risk(Some("1.49")),
            ("medium", Some("Health factor is below 1.5"))
        );
        assert_eq!(classify_liquidation_risk(Some("2.0")), ("low", None));
        assert_eq!(
            classify_liquidation_risk(Some("abc")),
            ("unknown", Some("Unable to parse health factor"))
        );
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({});
        let args: LendingRatesArgs = serde_json::from_value(json).expect("args should parse");
        assert!(args.asset.is_none());
        assert!(!args.simple_mode);

        let json = serde_json::json!({ "address": "0x1234567890123456789012345678901234567890" });
        let args: LiquidationRiskArgs = serde_json::from_value(json).expect("args should parse");
        assert_eq!(args.protocol, "tectonic");
        assert!(!args.simple_mode);
    }

    #[test]
    fn liquidation_args_deserialize_with_protocol() {
        let json = serde_json::json!({
            "address": "0x1234567890123456789012345678901234567890",
            "protocol": "tectonic",
            "simple_mode": true
        });
        let args: LiquidationRiskArgs = serde_json::from_value(json).expect("args should parse");
        assert_eq!(args.protocol, "tectonic");
        assert!(args.simple_mode);
    }
}
