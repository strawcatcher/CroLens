use serde::Deserialize;
use serde_json::Value;
use worker::d1::D1Type;

use crate::error::{CroLensError, Result};
use crate::infra;

#[derive(Debug, Deserialize)]
struct SearchArgs {
    query: String,
    #[serde(default = "default_limit")]
    limit: u8,
}

fn default_limit() -> u8 {
    20
}

fn normalize_limit(limit: u8) -> i64 {
    limit.clamp(1, 50) as i64
}

fn validate_search_query(query: &str) -> Result<String> {
    let q = query.trim();
    if q.is_empty() {
        return Err(CroLensError::invalid_params(
            "query must not be empty".to_string(),
        ));
    }
    if q.chars().count() > 200 {
        return Err(CroLensError::invalid_params(
            "query too long (max 200 characters)".to_string(),
        ));
    }
    Ok(q.to_string())
}

fn build_like_pattern(query: &str) -> String {
    let escaped = query.replace('%', "\\%").replace('_', "\\_");
    format!("%{}%", escaped)
}

pub async fn search_contract(services: &infra::Services, args: Value) -> Result<Value> {
    let input: SearchArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let limit = normalize_limit(input.limit);
    let q = validate_search_query(&input.query)?;

    let like = build_like_pattern(&q);
    let like_arg = D1Type::Text(&like);
    let limit_arg = D1Type::Integer(limit as i32);
    let statement = services
        .db
        .prepare(
            "SELECT address, name, type, protocol_id FROM contracts \
             WHERE name LIKE ?1 ESCAPE '\\' \
             ORDER BY name LIMIT ?2",
        )
        .bind_refs([&like_arg, &limit_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let result = infra::db::run("search_contract", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(serde_json::json!({
            "name": row.get("name").and_then(|v| v.as_str()),
            "address": row.get("address").and_then(|v| v.as_str()),
            "type": row.get("type").and_then(|v| v.as_str()),
            "protocol": row.get("protocol_id").and_then(|v| v.as_str()),
        }));
    }

    Ok(serde_json::json!({ "results": out, "meta": services.meta() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limit_is_20() {
        assert_eq!(default_limit(), 20);
    }

    #[test]
    fn normalize_limit_clamps() {
        assert_eq!(normalize_limit(0), 1);
        assert_eq!(normalize_limit(1), 1);
        assert_eq!(normalize_limit(50), 50);
        assert_eq!(normalize_limit(100), 50);
    }

    #[test]
    fn validate_search_query_rejects_empty() {
        let err = validate_search_query("   ").unwrap_err();
        assert!(matches!(err, CroLensError::InvalidParams(_)));
    }

    #[test]
    fn validate_search_query_rejects_long() {
        let q = "a".repeat(201);
        let err = validate_search_query(&q).unwrap_err();
        assert!(matches!(err, CroLensError::InvalidParams(_)));
    }

    #[test]
    fn validate_search_query_trims() {
        let q = validate_search_query("  VVS Router  ").expect("should trim");
        assert_eq!(q, "VVS Router");
    }

    #[test]
    fn build_like_pattern_escapes_wildcards() {
        assert_eq!(build_like_pattern("a%_b"), "%a\\%\\_b%");
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({ "query": "router" });
        let args: SearchArgs = serde_json::from_value(json).expect("args should parse");
        assert_eq!(args.limit, 20);
    }

    #[test]
    fn args_deserialize_with_limit() {
        let json = serde_json::json!({ "query": "router", "limit": 5 });
        let args: SearchArgs = serde_json::from_value(json).expect("args should parse");
        assert_eq!(args.limit, 5);
    }
}
