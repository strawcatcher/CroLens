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

pub async fn search_contract(services: &infra::Services, args: Value) -> Result<Value> {
    let input: SearchArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let limit = input.limit.clamp(1, 50) as i64;
    let q = input.query.trim();
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

    let like = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
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
