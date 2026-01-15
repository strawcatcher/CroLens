use serde::{Deserialize, Serialize};
use worker::{
    console_error, console_log, console_warn, Context, Env, Method, Request, Response,
    ScheduledEvent,
};

mod abi;
mod adapters;
mod domain;
pub mod error;
pub mod gateway;
mod http;
mod infra;
pub mod mcp;
pub mod types;

use crate::error::CroLensError;
use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse};

const MAX_REQUEST_BODY_BYTES: usize = 10 * 1024;
const JSONRPC_IP_RATE_LIMIT_DEFAULT: u32 = 120;
const JSONRPC_IP_RATE_WINDOW_SECS_DEFAULT: u64 = 60;
const PRICE_SYNC_NEXT_RUN_KEY: &str = "cron:price_sync:next_run_ms";
const PRICE_SYNC_RETRY_STATE_KEY: &str = "cron:price_sync:retry_state";
const PRICE_SYNC_BASE_INTERVAL_MS: i64 = 5 * 60 * 1000;
const PRICE_SYNC_RETRY_DELAYS_MS: [i64; 3] = [60_000, 120_000, 240_000];

#[derive(Debug, Serialize, Deserialize)]
struct PriceSyncRetryState {
    retries_done: u8,
    next_retry_ms: i64,
}

#[worker::event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> worker::Result<Response> {
    console_error_panic_hook::set_once();

    let trace_id = types::get_trace_id(&req);
    let start_ms = types::now_ms();
    let origin = types::get_header(&req, "Origin");

    let mut resp = match (req.method(), req.path().as_str()) {
        (Method::Options, _) => Response::ok("")?.with_status(204),
        (Method::Get, "/health") => handle_health(&env).await?,
        (Method::Get, "/ready") => handle_ready(&env).await?,
        (Method::Get, "/stats") => http::handle_stats(&env, &trace_id, start_ms).await?,
        (Method::Get, "/x402/quote") => {
            http::handle_x402_quote(&req, &env, &trace_id, start_ms).await?
        }
        (Method::Get, "/x402/status") => {
            http::handle_x402_status(&req, &env, &trace_id, start_ms).await?
        }
        (Method::Post, "/x402/verify") => {
            http::handle_x402_verify(req, &env, &trace_id, start_ms).await?
        }
        (Method::Post, "/") => handle_json_rpc(req, &env, &trace_id).await?,
        (Method::Post, "/_internal/price-sync") => handle_price_sync(&env).await?,
        (Method::Get, "/_internal/test-coingecko") => handle_test_coingecko().await?,
        _ => Response::error("Not Found", 404)?,
    };

    http::add_security_headers(resp.headers_mut())?;
    apply_cors(resp, &env, origin.as_deref())
}

#[worker::event(scheduled)]
pub async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: worker::ScheduleContext) {
    console_error_panic_hook::set_once();

    run_price_sync(&env).await;
}

async fn handle_price_sync(env: &Env) -> worker::Result<Response> {
    let mut messages = Vec::new();

    messages.push("Starting anchor price sync...".to_string());
    match infra::price::update_anchor_prices(env).await {
        Ok(_) => {
            messages.push("Anchor price sync succeeded".to_string());
        }
        Err(err) => {
            messages.push(format!("Anchor price sync failed: {err}"));
        }
    }

    // 检查 anchor 价格是否已写入
    if let Ok(kv) = env.kv("KV") {
        if let Ok(Some(v)) = kv.get("price:anchor:cro").text().await {
            messages.push(format!("CRO price in KV: {v}"));
        } else {
            messages.push("CRO price NOT in KV".to_string());
        }
    }

    messages.push("Starting derived price sync...".to_string());
    match infra::price::update_derived_prices(env).await {
        Ok(_) => {
            messages.push("Derived price sync succeeded".to_string());
        }
        Err(err) => {
            messages.push(format!("Derived price sync failed: {err}"));
        }
    }

    // 检查聚合缓存
    if let Ok(kv) = env.kv("KV") {
        if let Ok(Some(v)) = kv.get("cache:prices:all").text().await {
            messages.push(format!("Price cache: {} bytes", v.len()));
        }
    }

    Response::ok(messages.join("\n"))
}

async fn handle_test_coingecko() -> worker::Result<Response> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=crypto-com-chain&vs_currencies=usd";

    let mut headers = worker::Headers::new();
    headers.set("User-Agent", "CroLens/1.0 (https://crolens.io)")?;
    headers.set("Accept", "application/json")?;

    let req = worker::Request::new_with_init(
        url,
        worker::RequestInit::new()
            .with_method(worker::Method::Get)
            .with_headers(headers),
    )?;

    let mut resp = worker::Fetch::Request(req).send().await?;
    let text = resp.text().await?;

    Response::ok(format!("Status: {}, Body: {}", resp.status_code(), text))
}

async fn handle_json_rpc(mut req: Request, env: &Env, trace_id: &str) -> worker::Result<Response> {
    let start_ms = types::now_ms();
    let api_key = types::get_header(&req, "x-api-key");
    let client_ip = types::get_client_ip(&req);

    // 先解析请求体，这样可以判断是否需要 rate limit
    let body_bytes = match req.bytes().await {
        Ok(bytes) => bytes,
        Err(err) => {
            let resp = JsonRpcResponse::error(
                serde_json::Value::Null,
                CroLensError::invalid_request(format!("Failed to read request body: {err}")),
            );
            return Response::from_json(&resp).map(|r| r.with_status(400));
        }
    };
    if body_bytes.len() > MAX_REQUEST_BODY_BYTES {
        let resp = JsonRpcResponse::error(
            serde_json::Value::Null,
            CroLensError::invalid_request("Request body too large".to_string()),
        );
        return Response::from_json(&resp).map(|r| r.with_status(413));
    }

    let json_rpc_req: JsonRpcRequest = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(err) => {
            let resp = JsonRpcResponse::error(
                serde_json::Value::Null,
                CroLensError::invalid_request(format!("Invalid JSON-RPC payload: {err}")),
            );
            return Response::from_json(&resp).map(|r| r.with_status(400));
        }
    };

    console_log!(
        "[INFO] [{}] {} {}",
        trace_id,
        json_rpc_req.method,
        req.path()
    );

    // 对于只读的元数据请求，跳过 IP rate limit 以减少 KV 延迟
    // tools/call 内部有自己的 API key rate limit
    let needs_ip_rate_limit = json_rpc_req.method == "tools/call";

    if needs_ip_rate_limit {
        if let Ok(kv) = env.kv("KV") {
            let limit = env
                .var("RATE_LIMIT_JSONRPC_PER_MIN")
                .ok()
                .and_then(|v| v.to_string().parse::<u32>().ok())
                .filter(|v| *v > 0)
                .unwrap_or(JSONRPC_IP_RATE_LIMIT_DEFAULT);
            let window_secs = env
                .var("RATE_LIMIT_JSONRPC_WINDOW_SECS")
                .ok()
                .and_then(|v| v.to_string().parse::<u64>().ok())
                .filter(|v| *v > 0)
                .unwrap_or(JSONRPC_IP_RATE_WINDOW_SECS_DEFAULT);

            let key = format!("rl:jsonrpc:{client_ip}");
            match gateway::ratelimit::check_rate_limit(&kv, &key, limit, window_secs).await {
                Ok(true) => {}
                Ok(false) => {
                    let resp = JsonRpcResponse::error(
                        json_rpc_req.id,
                        CroLensError::rate_limit_exceeded(Some(window_secs as u32)),
                    );
                    let mut http_resp = Response::from_json(&resp)?.with_status(429);
                    http_resp
                        .headers_mut()
                        .set("Retry-After", &window_secs.to_string())?;
                    return Ok(http_resp);
                }
                Err(err) => {
                    console_warn!("[WARN] JSON-RPC rate limit skipped: {}", err);
                }
            }
        }
    }

    let request_size = body_bytes.len();
    let resp = mcp::router::handle(
        json_rpc_req,
        env,
        trace_id,
        api_key.as_deref(),
        start_ms,
        &client_ip,
        request_size,
    )
    .await;

    let mut http_resp = Response::from_json(&resp)?;
    if let Some(err) = resp.error.as_ref() {
        match err.code {
            -32003 => {
                http_resp = http_resp.with_status(429);
                let retry_after = err
                    .data
                    .as_ref()
                    .and_then(|v| v.get("retry_after"))
                    .and_then(|v| v.as_i64())
                    .filter(|v| *v > 0)
                    .unwrap_or(3600);
                http_resp
                    .headers_mut()
                    .set("Retry-After", &retry_after.to_string())?;
            }
            -32001 => {
                http_resp = http_resp.with_status(401);
            }
            -32002 => {
                http_resp = http_resp.with_status(402);
            }
            -32501 => {
                http_resp = http_resp.with_status(503);
                if let Some(retry_after) = err
                    .data
                    .as_ref()
                    .and_then(|v| v.get("retry_after"))
                    .and_then(|v| v.as_i64())
                {
                    http_resp
                        .headers_mut()
                        .set("Retry-After", &retry_after.to_string())?;
                }
            }
            -32601 => {
                http_resp = http_resp.with_status(404);
            }
            -32600 | -32602 => {
                http_resp = http_resp.with_status(400);
            }
            _ => {
                http_resp = http_resp.with_status(500);
            }
        }
    }

    Ok(http_resp)
}

async fn run_price_sync(env: &Env) {
    let kv = match env.kv("KV") {
        Ok(v) => v,
        Err(err) => {
            console_error!("Price sync skipped: KV binding missing: {}", err);
            return;
        }
    };

    let now = types::now_ms();
    let next_run_ms = kv
        .get(PRICE_SYNC_NEXT_RUN_KEY)
        .text()
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok());

    let retry_state = kv
        .get(PRICE_SYNC_RETRY_STATE_KEY)
        .text()
        .await
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<PriceSyncRetryState>(&raw).ok());

    if let Some(state) = retry_state {
        if now < state.next_retry_ms {
            return;
        }

        let attempt = state.retries_done.saturating_add(1);
        console_log!("[INFO] Price sync retry attempt {}", attempt);

        match infra::price::update_anchor_prices(env).await {
            Ok(_) => {
                console_log!("[INFO] Anchor price sync succeeded on retry {}", attempt);
                // anchor 价格更新成功后，立即更新 derived 价格
                match infra::price::update_derived_prices(env).await {
                    Ok(_) => {
                        console_log!("[INFO] Derived price sync succeeded on retry {}", attempt);
                    }
                    Err(err) => {
                        console_warn!("[WARN] Derived price sync failed on retry {}: {}", attempt, err);
                    }
                }
                let _ = kv.delete(PRICE_SYNC_RETRY_STATE_KEY).await;
                set_price_sync_next_run(&kv, now.saturating_add(PRICE_SYNC_BASE_INTERVAL_MS)).await;
            }
            Err(err) => {
                console_error!("[WARN] Price sync retry {} failed: {}", attempt, err);

                if attempt >= 3 {
                    console_error!("[ERROR] Price sync exhausted retries: {}", err);
                    let _ = kv.delete(PRICE_SYNC_RETRY_STATE_KEY).await;
                    set_price_sync_next_run(&kv, now.saturating_add(PRICE_SYNC_BASE_INTERVAL_MS))
                        .await;
                    return;
                }

                let delay_ms = PRICE_SYNC_RETRY_DELAYS_MS
                    .get(state.retries_done as usize)
                    .copied()
                    .unwrap_or(240_000);
                let next_state = PriceSyncRetryState {
                    retries_done: attempt,
                    next_retry_ms: now.saturating_add(delay_ms),
                };
                set_price_sync_retry_state(&kv, &next_state).await;
            }
        }

        return;
    }

    if let Some(next_run_ms) = next_run_ms {
        if now < next_run_ms {
            return;
        }
    }

    console_log!("[INFO] Price sync scheduled run");
    match infra::price::update_anchor_prices(env).await {
        Ok(_) => {
            console_log!("[INFO] Anchor price sync succeeded");
            // anchor 价格更新成功后，立即更新 derived 价格
            match infra::price::update_derived_prices(env).await {
                Ok(_) => {
                    console_log!("[INFO] Derived price sync succeeded");
                }
                Err(err) => {
                    console_warn!("[WARN] Derived price sync failed: {}", err);
                }
            }
            set_price_sync_next_run(&kv, now.saturating_add(PRICE_SYNC_BASE_INTERVAL_MS)).await;
        }
        Err(err) => {
            console_error!("[WARN] Anchor price sync failed: {}", err);
            let state = PriceSyncRetryState {
                retries_done: 0,
                next_retry_ms: now.saturating_add(PRICE_SYNC_RETRY_DELAYS_MS[0]),
            };
            set_price_sync_retry_state(&kv, &state).await;
            set_price_sync_next_run(&kv, now.saturating_add(PRICE_SYNC_BASE_INTERVAL_MS)).await;
        }
    }
}

async fn set_price_sync_next_run(kv: &worker::kv::KvStore, next_run_ms: i64) {
    if let Ok(put) = kv.put(PRICE_SYNC_NEXT_RUN_KEY, next_run_ms.to_string()) {
        let _ = put.expiration_ttl(86_400).execute().await;
    }
}

async fn set_price_sync_retry_state(kv: &worker::kv::KvStore, state: &PriceSyncRetryState) {
    let Ok(raw) = serde_json::to_string(state) else {
        return;
    };
    if let Ok(put) = kv.put(PRICE_SYNC_RETRY_STATE_KEY, raw) {
        let _ = put.expiration_ttl(1_800).execute().await;
    }
}

/// Readiness probe - checks if the service is ready to accept traffic
/// This is a lightweight check that only verifies the DB connection.
/// Use /health for a comprehensive health check including RPC.
async fn handle_ready(env: &Env) -> worker::Result<Response> {
    let (db_ok, db_error) = match env.d1("DB") {
        Ok(db) => match db.prepare("SELECT 1").all().await {
            Ok(_) => (true, None),
            Err(err) => (false, Some(err.to_string())),
        },
        Err(err) => (false, Some(err.to_string())),
    };

    if db_ok {
        Response::from_json(&serde_json::json!({
            "status": "ready",
            "version": env!("CARGO_PKG_VERSION"),
        }))
    } else {
        Response::from_json(&serde_json::json!({
            "status": "not_ready",
            "error": db_error,
        }))
        .map(|r| r.with_status(503))
    }
}

/// Liveness probe - comprehensive health check of all dependencies
async fn handle_health(env: &Env) -> worker::Result<Response> {
    let now = types::now_ms();

    let db_started = types::now_ms();
    let (db_ok, db_error) = match env.d1("DB") {
        Ok(db) => match db.prepare("SELECT 1").all().await {
            Ok(_) => (true, None),
            Err(err) => (false, Some(err.to_string())),
        },
        Err(err) => (false, Some(err.to_string())),
    };
    let db_latency_ms = types::now_ms().saturating_sub(db_started);

    let kv_started = types::now_ms();
    let (kv_ok, kv_error) = match env.kv("KV") {
        Ok(kv) => match kv.get("health:ping").text().await {
            Ok(_) => (true, None),
            Err(err) => (false, Some(err.to_string())),
        },
        Err(err) => (false, Some(err.to_string())),
    };
    let kv_latency_ms = types::now_ms().saturating_sub(kv_started);

    let mut rpc_ok = false;
    let mut rpc_latency_ms = 0i64;
    let mut rpc_error: Option<String> = None;

    let rpc_url = env.var("BLOCKPI_RPC_URL").ok().map(|v| v.to_string());
    if let Some(url) = rpc_url
        .as_deref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
    {
        let rpc_started = types::now_ms();
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_blockNumber",
            "params": []
        });
        let body = match serde_json::to_string(&payload) {
            Ok(v) => v,
            Err(err) => {
                rpc_error = Some(err.to_string());
                String::new()
            }
        };

        if rpc_error.is_none() {
            let headers = worker::Headers::new();
            headers.set("Content-Type", "application/json")?;

            let mut init = worker::RequestInit::new();
            init.with_method(worker::Method::Post);
            init.with_headers(headers);
            init.with_body(Some(body.into()));

            match worker::Request::new_with_init(url, &init) {
                Ok(request) => match worker::Fetch::Request(request).send().await {
                    Ok(mut resp) => match resp.json::<serde_json::Value>().await {
                        Ok(value) => {
                            if value.get("result").is_some() && value.get("error").is_none() {
                                rpc_ok = true;
                            } else {
                                rpc_error = Some("Invalid RPC response".to_string());
                            }
                        }
                        Err(err) => {
                            rpc_error = Some(err.to_string());
                        }
                    },
                    Err(err) => {
                        rpc_error = Some(err.to_string());
                    }
                },
                Err(err) => {
                    rpc_error = Some(err.to_string());
                }
            }
        }

        rpc_latency_ms = types::now_ms().saturating_sub(rpc_started);
    } else {
        rpc_error = Some("Missing env var: BLOCKPI_RPC_URL".to_string());
    }

    let overall_status = if !db_ok {
        "unhealthy"
    } else if !kv_ok || !rpc_ok {
        "degraded"
    } else {
        "ok"
    };

    let payload = serde_json::json!({
        "status": overall_status,
        "version": env!("CARGO_PKG_VERSION"),
        "checks": {
            "db": {
                "status": if db_ok { "ok" } else { "error" },
                "latency_ms": db_latency_ms,
                "error": db_error,
            },
            "kv": {
                "status": if kv_ok { "ok" } else { "error" },
                "latency_ms": kv_latency_ms,
                "error": kv_error,
            },
            "rpc": {
                "status": if rpc_ok { "ok" } else { "error" },
                "latency_ms": rpc_latency_ms,
                "error": rpc_error,
            },
        },
        "timestamp": now,
    });

    let status_code = if overall_status == "ok" { 200 } else { 503 };
    Response::from_json(&payload).map(|r| r.with_status(status_code))
}

fn apply_cors(mut resp: Response, env: &Env, origin: Option<&str>) -> worker::Result<Response> {
    let headers = resp.headers_mut();
    let configured = env
        .var("CORS_ALLOW_ORIGIN")
        .ok()
        .map(|v| v.to_string())
        .unwrap_or_default();
    let configured = configured.trim();

    if configured.is_empty() {
        if let Some(origin) = origin {
            console_error!("[WARN] CORS rejected for origin {}", origin);
            return Response::error("CORS forbidden", 403);
        }
        return Ok(resp);
    }

    if configured == "*" {
        headers.set("Access-Control-Allow-Origin", "*")?;
    } else {
        let allowed = configured
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if allowed.is_empty() {
            return Ok(resp);
        }

        if let Some(origin) = origin {
            if allowed.iter().any(|v| v.eq_ignore_ascii_case(origin)) {
                headers.set("Access-Control-Allow-Origin", origin)?;
                headers.set("Vary", "Origin")?;
            } else {
                console_error!("[WARN] CORS rejected for origin {}", origin);
                return Response::error("CORS forbidden", 403);
            }
        }
    }

    headers.set("Access-Control-Allow-Methods", "GET,POST,OPTIONS")?;
    headers.set(
        "Access-Control-Allow-Headers",
        "Content-Type,x-api-key,x-request-id",
    )?;
    headers.set("Access-Control-Max-Age", "86400")?;
    Ok(resp)
}
