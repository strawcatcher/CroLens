use alloy_primitives::{Address, U256};
use serde_json::Value;
use worker::{Fetch, Headers, Method, Request, RequestInit};

use crate::error::{CroLensError, Result};
use crate::types;

#[derive(Clone)]
pub struct TenderlyClient {
    access_key: String,
    account: String,
    project: String,
}

#[derive(Debug, Clone)]
pub struct TenderlySimulation {
    pub success: bool,
    pub gas_used: Option<u64>,
    pub logs: Vec<TenderlyLog>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TenderlyLog {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
}

impl TenderlyClient {
    pub fn try_new(env: &worker::Env) -> Option<Self> {
        let access_key = env
            .var("TENDERLY_ACCESS_KEY")
            .ok()
            .map(|v| v.to_string())
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                env.var("TENDERLY_API_KEY")
                    .ok()
                    .map(|v| v.to_string())
                    .filter(|v| !v.trim().is_empty())
            })?;

        let account = env
            .var("TENDERLY_ACCOUNT")
            .ok()
            .map(|v| v.to_string())
            .filter(|v| !v.trim().is_empty())?;

        let project = env
            .var("TENDERLY_PROJECT")
            .ok()
            .map(|v| v.to_string())
            .filter(|v| !v.trim().is_empty())?;

        Some(Self {
            access_key,
            account,
            project,
        })
    }

    pub async fn simulate(
        &self,
        from: Address,
        to: Address,
        input: &str,
        value: U256,
    ) -> Result<TenderlySimulation> {
        let url = format!(
            "https://api.tenderly.co/api/v1/account/{}/project/{}/simulate",
            self.account, self.project
        );

        let body = serde_json::json!({
            "network_id": "25",
            "from": from.to_string(),
            "to": to.to_string(),
            "input": input,
            "value": value.to_string(),
            "save": false,
            "save_if_fails": false,
            "simulation_type": "quick"
        });

        let body_str = serde_json::to_string(&body)
            .map_err(|err| CroLensError::SimulationFailed(err.to_string()))?;

        let headers = Headers::new();
        headers
            .set("Content-Type", "application/json")
            .map_err(|err| CroLensError::SimulationFailed(err.to_string()))?;
        headers
            .set("X-Access-Key", &self.access_key)
            .map_err(|err| CroLensError::SimulationFailed(err.to_string()))?;

        let mut init = RequestInit::new();
        init.with_method(Method::Post);
        init.with_headers(headers);
        init.with_body(Some(body_str.into()));

        let request = Request::new_with_init(&url, &init)
            .map_err(|err| CroLensError::SimulationFailed(err.to_string()))?;

        let mut resp = Fetch::Request(request)
            .send()
            .await
            .map_err(|err| CroLensError::SimulationFailed(err.to_string()))?;

        let status_code = resp.status_code();
        let payload: Value = resp
            .json()
            .await
            .map_err(|err| CroLensError::SimulationFailed(err.to_string()))?;

        if status_code >= 400 {
            return Err(CroLensError::SimulationFailed(format!(
                "Tenderly HTTP {status_code}: {payload}"
            )));
        }

        let tx = payload.get("transaction").cloned().unwrap_or(Value::Null);
        let success = tx.get("status").and_then(|v| v.as_bool()).unwrap_or(false);
        let gas_used = tx.get("gas_used").and_then(|v| v.as_u64());
        let error_message = tx
            .get("error_message")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());

        let logs_value = tx
            .get("transaction_info")
            .and_then(|v| v.get("logs"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut logs = Vec::with_capacity(logs_value.len());
        for item in logs_value {
            let address = item
                .get("address")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let topics = item
                .get("topics")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let data = item
                .get("data")
                .and_then(|v| v.as_str())
                .unwrap_or("0x")
                .to_string();

            logs.push(TenderlyLog {
                address: normalize_address(&address),
                topics: topics
                    .into_iter()
                    .map(|t| normalize_hex(&t))
                    .collect::<Vec<_>>(),
                data: normalize_hex(&data),
            });
        }

        Ok(TenderlySimulation {
            success,
            gas_used,
            logs,
            error_message,
        })
    }
}

fn normalize_hex(value: &str) -> String {
    if value.trim().starts_with("0x") {
        value.trim().to_lowercase()
    } else {
        format!("0x{}", value.trim().to_lowercase())
    }
}

fn normalize_address(value: &str) -> String {
    match types::parse_address(value) {
        Ok(addr) => addr.to_string(),
        Err(_) => value.to_string(),
    }
}
