use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct ResolveArgs {
    query: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn resolve_cronos_id(services: &infra::Services, args: Value) -> Result<Value> {
    let input: ResolveArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let q = input.query.trim();
    if q.is_empty() {
        return Err(CroLensError::invalid_params(
            "query must not be empty".to_string(),
        ));
    }

    let (name, address) = if q.ends_with(".cro") {
        (Some(q.to_string()), None)
    } else if let Ok(addr) = types::parse_address(q) {
        (None, Some(addr.to_string()))
    } else {
        (Some(q.to_string()), None)
    };

    if input.simple_mode {
        let text = match (&name, &address) {
            (Some(n), _) => format!("Cronos ID lookup: {n} (placeholder)"),
            (_, Some(a)) => format!("Reverse lookup: {a} (placeholder)"),
            _ => "Cronos ID lookup (placeholder).".to_string(),
        };
        return Ok(serde_json::json!({ "text": text, "meta": services.meta() }));
    }

    Ok(serde_json::json!({
        "query": input.query,
        "name": name,
        "address": address,
        "meta": services.meta(),
    }))
}

