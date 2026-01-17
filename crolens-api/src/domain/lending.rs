use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

fn default_protocol() -> String {
    "tectonic".to_string()
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

    let (risk_level, warning) = match health_factor.as_deref() {
        Some("∞") => ("low", None),
        Some(v) => match v.parse::<f64>() {
            Ok(hf) if hf < 1.1 => ("high", Some("Health factor is below 1.1")),
            Ok(hf) if hf < 1.5 => ("medium", Some("Health factor is below 1.5")),
            Ok(_) => ("low", None),
            Err(_) => ("unknown", Some("Unable to parse health factor")),
        },
        None => ("unknown", Some("Health factor unavailable")),
    };

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

