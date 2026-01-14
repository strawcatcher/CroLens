use alloy_primitives::U256;
use serde::Deserialize;
use worker::d1::D1Type;
use worker::{Env, Headers, Request, Response};

use crate::error::{CroLensError, Result};
use crate::gateway;
use crate::infra;
use crate::types;

const MAX_REQUEST_BODY_BYTES: usize = 10 * 1024;

pub fn add_security_headers(headers: &mut Headers) -> std::result::Result<(), worker::Error> {
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("X-Frame-Options", "DENY")?;
    headers.set("X-XSS-Protection", "1; mode=block")?;
    headers.set(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains",
    )?;
    headers.set(
        "Content-Security-Policy",
        "default-src 'none'; frame-ancestors 'none'",
    )?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct VerifyPaymentRequest {
    tx_hash: String,
}

pub async fn handle_stats(env: &Env, trace_id: &str, start_ms: i64) -> worker::Result<Response> {
    let db = env.d1("DB")?;

    let statement = db.prepare("SELECT COUNT(*) AS cnt FROM protocols WHERE is_active = 1");
    let result = infra::db::run("stats_count_protocols", statement.all())
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))?;
    let rows: Vec<serde_json::Value> = result.results()?;
    let protocols_supported = rows
        .first()
        .and_then(|v| v.get("cnt"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    Response::from_json(&serde_json::json!({
        "protocols_supported": protocols_supported,
        "meta": meta(trace_id, start_ms),
    }))
}

pub async fn handle_x402_quote(
    req: &Request,
    env: &Env,
    trace_id: &str,
    start_ms: i64,
) -> worker::Result<Response> {
    let kv = env.kv("KV")?;
    let ip = types::get_client_ip(req);
    let key = format!("rl:quote:{ip}");
    let allowed = gateway::ratelimit::check_rate_limit(&kv, &key, 30, 60)
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))?;
    if !allowed {
        let mut resp = Response::from_json(&serde_json::json!({
            "error": { "message": "Rate limit exceeded" },
            "meta": meta(trace_id, start_ms),
        }))?
        .with_status(429);
        resp.headers_mut().set("Retry-After", "60")?;
        return Ok(resp);
    }

    let db = env.d1("DB")?;
    let cfg = infra::x402::X402Config::try_load(env, &db)
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))?;

    let Some(cfg) = cfg else {
        return Response::from_json(&serde_json::json!({
            "error": { "message": "x402 is not configured (missing X402_PAYMENT_ADDRESS)" },
            "meta": meta(trace_id, start_ms),
        }))
        .map(|r| r.with_status(400));
    };

    let amount = cfg.topup_amount_wei();
    Response::from_json(&serde_json::json!({
        "chain_id": 25,
        "payment_address": cfg.payment_address.to_string(),
        "credits": cfg.topup_credits,
        "amount_wei": amount.to_string(),
        "price_per_credit_wei": cfg.price_per_credit_wei.to_string(),
        "meta": meta(trace_id, start_ms),
    }))
}

pub async fn handle_x402_status(
    req: &Request,
    env: &Env,
    trace_id: &str,
    start_ms: i64,
) -> worker::Result<Response> {
    let api_key = types::get_header(req, "x-api-key").unwrap_or_default();
    if api_key.trim().is_empty() {
        return Response::from_json(&serde_json::json!({
            "error": { "message": "Missing x-api-key" },
            "meta": meta(trace_id, start_ms),
        }))
        .map(|r| r.with_status(400));
    }

    let db = env.d1("DB")?;
    let record = match gateway::ensure_api_key(&db, &api_key, None).await {
        Ok(v) => v,
        Err(CroLensError::Unauthorized(msg)) => {
            return Response::from_json(&serde_json::json!({
                "error": { "message": msg },
                "meta": meta(trace_id, start_ms),
            }))
            .map(|r| r.with_status(401));
        }
        Err(err) => return Err(worker::Error::RustError(err.to_string())),
    };

    Response::from_json(&serde_json::json!({
        "api_key": record.api_key,
        "tier": record.tier,
        "credits": record.credits,
        "meta": meta(trace_id, start_ms),
    }))
}

pub async fn handle_x402_verify(
    mut req: Request,
    env: &Env,
    trace_id: &str,
    start_ms: i64,
) -> worker::Result<Response> {
    let kv = env.kv("KV")?;
    let ip = types::get_client_ip(&req);
    let key = format!("rl:verify:{ip}");
    let allowed = gateway::ratelimit::check_rate_limit(&kv, &key, 10, 60)
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))?;
    if !allowed {
        let mut resp = Response::from_json(&serde_json::json!({
            "error": { "message": "Rate limit exceeded" },
            "meta": meta(trace_id, start_ms),
        }))?
        .with_status(429);
        resp.headers_mut().set("Retry-After", "60")?;
        return Ok(resp);
    }

    let api_key = types::get_header(&req, "x-api-key").unwrap_or_default();
    if api_key.trim().is_empty() {
        return Response::from_json(&serde_json::json!({
            "error": { "message": "Missing x-api-key" },
            "meta": meta(trace_id, start_ms),
        }))
        .map(|r| r.with_status(400));
    }
    if let Err(CroLensError::Unauthorized(msg)) = gateway::auth::validate_api_key_format(&api_key) {
        return Response::from_json(&serde_json::json!({
            "error": { "message": msg },
            "meta": meta(trace_id, start_ms),
        }))
        .map(|r| r.with_status(401));
    }

    let body_bytes = req.bytes().await?;
    if body_bytes.len() > MAX_REQUEST_BODY_BYTES {
        return Response::from_json(&serde_json::json!({
            "error": { "message": "Request body too large" },
            "meta": meta(trace_id, start_ms),
        }))
        .map(|r| r.with_status(413));
    }
    let body: VerifyPaymentRequest = serde_json::from_slice(&body_bytes).map_err(|err| {
        worker::Error::RustError(format!("Invalid JSON body for /x402/verify: {err}"))
    })?;
    let tx_hash = body.tx_hash.trim();
    if let Err(err) = types::validate_hex_string(tx_hash, 64) {
        return Response::from_json(&serde_json::json!({
            "error": { "message": err.to_string() },
            "meta": meta(trace_id, start_ms),
        }))
        .map(|r| r.with_status(400));
    }

    let db = env.d1("DB")?;
    let Some(cfg) = infra::x402::X402Config::try_load(env, &db)
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))?
    else {
        return Response::from_json(&serde_json::json!({
            "error": { "message": "x402 is not configured (missing X402_PAYMENT_ADDRESS)" },
            "meta": meta(trace_id, start_ms),
        }))
        .map(|r| r.with_status(400));
    };
    let amount_required = cfg.topup_amount_wei();

    let rpc = infra::rpc::RpcClient::try_new(env, Some(kv.clone()))
        .ok_or_else(|| worker::Error::RustError("Missing env var: BLOCKPI_RPC_URL".to_string()))?;

    let tx = rpc
        .eth_get_transaction_by_hash(tx_hash)
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))?;
    if tx.is_null() {
        return Response::from_json(&serde_json::json!({
            "status": "pending",
            "tx_hash": tx_hash,
            "meta": meta(trace_id, start_ms),
        }));
    }

    let receipt = rpc
        .eth_get_transaction_receipt(tx_hash)
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))?;
    if receipt.is_null() {
        return Response::from_json(&serde_json::json!({
            "status": "pending",
            "tx_hash": tx_hash,
            "meta": meta(trace_id, start_ms),
        }));
    }

    let status = receipt
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0");
    if status != "0x1" {
        return Response::from_json(&serde_json::json!({
            "status": "failed",
            "tx_hash": tx_hash,
            "error": { "message": "Transaction failed" },
            "meta": meta(trace_id, start_ms),
        }))
        .map(|r| r.with_status(400));
    }

    let to = tx.get("to").and_then(|v| v.as_str()).unwrap_or_default();
    if !to.eq_ignore_ascii_case(&cfg.payment_address.to_string()) {
        return Response::from_json(&serde_json::json!({
            "status": "rejected",
            "tx_hash": tx_hash,
            "error": { "message": "Transaction recipient mismatch" },
            "meta": meta(trace_id, start_ms),
        }))
        .map(|r| r.with_status(400));
    }

    let from = tx.get("from").and_then(|v| v.as_str()).unwrap_or_default();
    let value_hex = tx.get("value").and_then(|v| v.as_str()).unwrap_or("0x0");
    let value = types::parse_u256_hex(value_hex).unwrap_or(U256::ZERO);
    if value < amount_required {
        return Response::from_json(&serde_json::json!({
            "status": "rejected",
            "tx_hash": tx_hash,
            "error": { "message": "Payment amount too low" },
            "meta": meta(trace_id, start_ms),
        }))
        .map(|r| r.with_status(400));
    }

    let inserted = insert_payment_once(&db, tx_hash, &api_key, from, to, &value, cfg.topup_credits)
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))?;

    if inserted {
        gateway::grant_credits(&db, &api_key, Some(from), cfg.topup_credits, "pro")
            .await
            .map_err(|err| worker::Error::RustError(err.to_string()))?;
    }

    let record = gateway::lookup_api_key(&db, &api_key)
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))?
        .unwrap_or(gateway::ApiKeyRecord {
            api_key,
            tier: "free".to_string(),
            credits: 0,
            is_active: true,
        });

    Response::from_json(&serde_json::json!({
        "status": if inserted { "credited" } else { "already_credited" },
        "tx_hash": tx_hash,
        "credits_added": if inserted { cfg.topup_credits } else { 0 },
        "credits": record.credits,
        "tier": record.tier,
        "meta": meta(trace_id, start_ms),
    }))
}

async fn insert_payment_once(
    db: &worker::D1Database,
    tx_hash: &str,
    api_key: &str,
    from: &str,
    to: &str,
    value: &U256,
    credits: i64,
) -> Result<bool> {
    let tx_arg = D1Type::Text(tx_hash);
    let api_key_arg = D1Type::Text(api_key);
    let from_arg = if from.trim().is_empty() {
        D1Type::Null
    } else {
        D1Type::Text(from)
    };
    let to_arg = if to.trim().is_empty() {
        D1Type::Null
    } else {
        D1Type::Text(to)
    };
    let value_arg = D1Type::Text(&value.to_string());
    let credits_arg = D1Type::Integer(credits.clamp(0, i32::MAX as i64) as i32);

    let statement = db
        .prepare(
            "INSERT INTO payments (tx_hash, api_key, from_address, to_address, value_wei, credits_granted) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind_refs([&tx_arg, &api_key_arg, &from_arg, &to_arg, &value_arg, &credits_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    match infra::db::run("insert_payment_once", statement.run()).await {
        Ok(_) => Ok(true),
        Err(CroLensError::DbError(msg)) => {
            if msg.contains("UNIQUE constraint failed") || msg.contains("SQLITE_CONSTRAINT") {
                Ok(false)
            } else {
                Err(CroLensError::DbError(msg))
            }
        }
        Err(err) => Err(err),
    }
}

fn meta(trace_id: &str, start_ms: i64) -> serde_json::Value {
    let now = types::now_ms();
    serde_json::json!({
        "trace_id": trace_id,
        "timestamp": now,
        "latency_ms": now.saturating_sub(start_ms),
    })
}
