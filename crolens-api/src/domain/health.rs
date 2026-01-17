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

fn approval_risk_alert(risk_score: u64) -> Option<Value> {
    if risk_score < 20 {
        return None;
    }
    Some(serde_json::json!({
        "category": "approvals",
        "level": "warning",
        "message": "High token approval risk score.",
    }))
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

        if let Some(alert) = approval_risk_alert(risk_score) {
            alerts.push(alert);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_risk_alert_threshold() {
        assert!(approval_risk_alert(0).is_none());
        assert!(approval_risk_alert(19).is_none());
        let alert = approval_risk_alert(20).expect("should emit alert");
        assert_eq!(alert.get("category").and_then(|v| v.as_str()), Some("approvals"));
        assert_eq!(alert.get("level").and_then(|v| v.as_str()), Some("warning"));
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({ "address": "0x1234567890123456789012345678901234567890" });
        let args: HealthAlertsArgs = serde_json::from_value(json).expect("args should parse");
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({
            "address": "0x1234567890123456789012345678901234567890",
            "simple_mode": true
        });
        let args: HealthAlertsArgs = serde_json::from_value(json).expect("args should parse");
        assert!(args.simple_mode);
    }
}
