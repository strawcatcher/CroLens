use serde::Deserialize;
use serde_json::Value;
use worker::d1::D1Type;

use crate::error::{CroLensError, Result};
use crate::infra;

#[derive(Debug, Deserialize)]
struct ProtocolStatsArgs {
    #[serde(default)]
    protocol: Option<String>,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_protocol_stats(services: &infra::Services, args: Value) -> Result<Value> {
    let input: ProtocolStatsArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let protocol = input
        .protocol
        .clone()
        .unwrap_or_else(|| "all".to_string());

    let (pool_count, market_count) = if protocol == "all" {
        (
            count_rows(&services.db, "dex_pools", None).await?,
            count_rows(&services.db, "lending_markets", None).await?,
        )
    } else {
        (
            count_rows(&services.db, "dex_pools", Some(&protocol)).await?,
            count_rows(&services.db, "lending_markets", Some(&protocol)).await?,
        )
    };

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": format!("Protocol stats ({protocol}): pools={pool_count}, markets={market_count}"),
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "protocol": protocol,
        "pool_count": pool_count,
        "market_count": market_count,
        "meta": services.meta(),
    }))
}

async fn count_rows(db: &worker::D1Database, table: &str, protocol: Option<&str>) -> Result<i64> {
    let sql = match protocol {
        Some(_) => format!("SELECT COUNT(*) AS cnt FROM {table} WHERE protocol_id = ?1"),
        None => format!("SELECT COUNT(*) AS cnt FROM {table}"),
    };

    let statement = db.prepare(&sql);
    let statement = match protocol {
        Some(p) => {
            let protocol_arg = D1Type::Text(p);
            statement
                .bind_refs([&protocol_arg])
                .map_err(|err| CroLensError::DbError(err.to_string()))?
        }
        None => statement,
    };
    let result = infra::db::run("get_protocol_stats_count", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let Some(row) = rows.first() else {
        return Ok(0);
    };
    Ok(row.get("cnt").and_then(|v| v.as_i64()).unwrap_or(0))
}
