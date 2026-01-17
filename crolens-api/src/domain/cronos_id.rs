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

fn validate_query(query: &str) -> Result<String> {
    let q = query.trim();
    if q.is_empty() {
        return Err(CroLensError::invalid_params(
            "query must not be empty".to_string(),
        ));
    }
    Ok(q.to_string())
}

fn is_cro_domain(query: &str) -> bool {
    query.ends_with(".cro")
}

fn classify_query(query: &str) -> (Option<String>, Option<String>) {
    if is_cro_domain(query) {
        return (Some(query.to_string()), None);
    }
    if let Ok(addr) = types::parse_address(query) {
        return (None, Some(addr.to_string()));
    }
    (Some(query.to_string()), None)
}

pub async fn resolve_cronos_id(services: &infra::Services, args: Value) -> Result<Value> {
    let input: ResolveArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let q = validate_query(&input.query)?;
    let (name, address) = classify_query(&q);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_query_rejects_empty() {
        let result = validate_query("   ");
        assert!(result.is_err());
    }

    #[test]
    fn validate_query_trims_whitespace() {
        let result = validate_query("  alice.cro  ").expect("should trim");
        assert_eq!(result, "alice.cro");
    }

    #[test]
    fn is_cro_domain_detects_suffix() {
        assert!(is_cro_domain("alice.cro"));
        assert!(!is_cro_domain("alice.eth"));
        assert!(!is_cro_domain("0x1234"));
    }

    #[test]
    fn classify_query_detects_address() {
        let (name, address) = classify_query("0x1234567890123456789012345678901234567890");
        assert!(name.is_none());
        assert_eq!(
            address.as_deref(),
            Some("0x1234567890123456789012345678901234567890")
        );
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({ "query": "test.cro" });
        let args: ResolveArgs = serde_json::from_value(json).expect("should parse");
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({ "query": "test.cro", "simple_mode": true });
        let args: ResolveArgs = serde_json::from_value(json).expect("should parse");
        assert!(args.simple_mode);
    }
}
