use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct HealthAlertsArgs {
    address: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_health_alerts(services: &infra::Services, args: Value) -> Result<Value> {
    let input: HealthAlertsArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let _ = types::parse_address(&input.address)?;

    let mut alerts: Vec<Value> = Vec::new();

    if let Ok(approvals) = crate::domain::approval::get_approval_status(
        services,
        serde_json::json!({ "address": input.address, "simple_mode": false }),
    )
    .await
    {
        let risk_score = approvals
            .get("summary")
            .and_then(|v| v.get("risk_score"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if risk_score >= 20 {
            alerts.push(serde_json::json!({
                "category": "approvals",
                "level": "warning",
                "message": "High token approval risk score.",
            }));
        }
    }

    if input.simple_mode {
        let text = if alerts.is_empty() {
            "No health alerts.".to_string()
        } else {
            format!("Health alerts: {}", alerts.len())
        };
        return Ok(serde_json::json!({ "text": text, "meta": services.meta() }));
    }

    Ok(serde_json::json!({
        "address": input.address,
        "alerts": alerts,
        "meta": services.meta(),
    }))
}

