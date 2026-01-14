use worker::d1::D1Type;
use worker::D1Database;

use crate::error::{CroLensError, Result};
use crate::infra;

pub async fn log_request(
    db: &D1Database,
    trace_id: &str,
    api_key: Option<&str>,
    tool_name: &str,
    latency_ms: i64,
    status: &str,
    error_code: Option<i32>,
    ip_address: Option<&str>,
    request_size: Option<usize>,
) -> Result<()> {
    let trace_arg = D1Type::Text(trace_id);
    let api_key_arg = match api_key {
        Some(v) => D1Type::Text(v),
        None => D1Type::Null,
    };
    let tool_arg = D1Type::Text(tool_name);
    let latency_arg = D1Type::Integer(latency_ms.clamp(0, i32::MAX as i64) as i32);
    let status_arg = D1Type::Text(status);
    let error_arg = match error_code {
        Some(v) => D1Type::Integer(v),
        None => D1Type::Null,
    };
    let ip_arg = match ip_address {
        Some(v) if !v.trim().is_empty() => D1Type::Text(v),
        _ => D1Type::Null,
    };
    let size_arg = match request_size {
        Some(v) => D1Type::Integer((v as i64).clamp(0, i32::MAX as i64) as i32),
        None => D1Type::Null,
    };

    let statement = db.prepare(
        "INSERT INTO request_logs (trace_id, api_key, tool_name, latency_ms, status, error_code, ip_address, request_size) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind_refs([
        &trace_arg,
        &api_key_arg,
        &tool_arg,
        &latency_arg,
        &status_arg,
        &error_arg,
        &ip_arg,
        &size_arg,
    ])
    .map_err(|err| CroLensError::DbError(err.to_string()))?;

    infra::db::run("log_request", statement.run()).await?;

    Ok(())
}
