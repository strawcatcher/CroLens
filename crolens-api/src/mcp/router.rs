use serde_json::Value;
use worker::{console_error, Env};

use crate::domain;
use crate::error::CroLensError;
use crate::gateway;
use crate::infra;
use crate::infra::structured_log::{LogEntry, LogLevel, RequestContext};
use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse, ToolCallParams};
use crate::types;

pub async fn handle(
    req: JsonRpcRequest,
    env: &Env,
    trace_id: &str,
    api_key: Option<&str>,
    start_ms: i64,
    client_ip: &str,
    request_size: usize,
) -> JsonRpcResponse {
    if req.jsonrpc != "2.0" {
        return JsonRpcResponse::error(
            req.id,
            CroLensError::invalid_request("jsonrpc must be '2.0'".to_string()),
        );
    }

    match req.method.as_str() {
        "tools/list" => JsonRpcResponse::success(req.id, crate::mcp::tools::list()),
        "tools/call" => {
            handle_tools_call(
                req,
                env,
                trace_id,
                api_key,
                start_ms,
                client_ip,
                request_size,
            )
            .await
        }
        _ => JsonRpcResponse::error(req.id, CroLensError::method_not_found(req.method)),
    }
}

async fn handle_tools_call(
    req: JsonRpcRequest,
    env: &Env,
    trace_id: &str,
    api_key: Option<&str>,
    start_ms: i64,
    client_ip: &str,
    request_size: usize,
) -> JsonRpcResponse {
    let params: ToolCallParams = match serde_json::from_value(req.params) {
        Ok(v) => v,
        Err(err) => {
            return JsonRpcResponse::error(
                req.id,
                CroLensError::invalid_params(format!("Invalid tools/call params: {err}")),
            )
        }
    };

    let db = match env.d1("DB") {
        Ok(v) => v,
        Err(err) => return JsonRpcResponse::error(req.id, CroLensError::DbError(err.to_string())),
    };

    let tool_name = params.name.clone();
    let outcome: std::result::Result<Value, CroLensError> = async {
        // 延迟加载 X402 配置，只在需要返回支付错误时才加载
        let lazy_payment_data = || async {
            match infra::x402::X402Config::try_load(env, &db).await {
                Ok(Some(cfg)) => Some(serde_json::json!({
                    "chain_id": 25,
                    "payment_address": cfg.payment_address.to_string(),
                    "price": format!("{} CRO", types::format_units(&cfg.topup_amount_wei(), 18)),
                    "credits": cfg.topup_credits,
                })),
                _ => None,
            }
        };

        let key = api_key.ok_or_else(|| {
            CroLensError::invalid_params("Missing API key header: x-api-key".to_string())
        })?;
        let record = gateway::ensure_api_key(&db, key, None).await?;

        let kv = env
            .kv("KV")
            .map_err(|err| CroLensError::KvError(err.to_string()))?;
        let limit = if record.tier == "pro" { 1000 } else { 50 };
        let rl_key = format!("rl:tool:{}", record.api_key);
        let allowed = gateway::ratelimit::check_rate_limit(&kv, &rl_key, limit, 3600).await?;
        if !allowed {
            return Err(CroLensError::rate_limit_exceeded(Some(3600)));
        }

        if record.credits <= 0 {
            return Err(CroLensError::payment_required(lazy_payment_data().await));
        }
        // Free 用户可以使用所有工具，后续再加限制
        gateway::deduct_credit(&db, &record.api_key).await?;

        let services = infra::Services::new(env, trace_id, start_ms)?;
        match tool_name.as_str() {
            "get_account_summary" => {
                domain::assets::get_account_summary(&services, params.arguments).await
            }
            "get_defi_positions" => {
                domain::defi::get_defi_positions(&services, params.arguments).await
            }
            "decode_transaction" => {
                domain::transaction::decode_transaction(&services, params.arguments).await
            }
            "simulate_transaction" => {
                domain::simulation::simulate_transaction(&services, params.arguments).await
            }
            "search_contract" => domain::search::search_contract(&services, params.arguments).await,
            "construct_swap_tx" => {
                domain::swap::construct_swap_tx(&services, params.arguments).await
            }
            _ => Err(CroLensError::method_not_found(format!(
                "Unknown tool: {tool_name}"
            ))),
        }
    }
    .await;

    let latency_ms = types::now_ms().saturating_sub(start_ms);
    let (status, error_code) = match &outcome {
        Ok(_) => ("success", None),
        Err(err) => {
            let (code, _, _) = err.to_json_rpc_error();
            ("error", Some(code))
        }
    };

    // Emit structured JSON log
    let log_ctx = RequestContext::new(trace_id, api_key, client_ip, start_ms);
    match &outcome {
        Ok(_) => log_ctx.log_request_complete(&tool_name, status),
        Err(err) => {
            let (code, msg, _) = err.to_json_rpc_error();
            log_ctx.log_request_error(&tool_name, code, &msg);
        }
    }

    let sample_rate = env
        .var("REQUEST_LOG_SAMPLE_RATE")
        .ok()
        .and_then(|v| v.to_string().parse::<f64>().ok())
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let should_log = status == "error" || should_sample(trace_id, sample_rate);
    if should_log {
        if let Err(err) = infra::logging::log_request(
            &db,
            trace_id,
            api_key,
            &tool_name,
            latency_ms,
            status,
            error_code,
            Some(client_ip),
            Some(request_size),
        )
        .await
        {
            console_error!("[WARN] request log write failed: {}", err);
        }
    }

    match outcome {
        Ok(value) => JsonRpcResponse::success(req.id, value),
        Err(err) => JsonRpcResponse::error(req.id, err),
    }
}

fn should_sample(trace_id: &str, sample_rate: f64) -> bool {
    if sample_rate >= 1.0 {
        return true;
    }
    if sample_rate <= 0.0 {
        return false;
    }

    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    trace_id.hash(&mut hasher);
    let v = hasher.finish();

    // 0..9999 bucket for stable sampling.
    let bucket = (v % 10_000) as f64 / 10_000.0;
    bucket < sample_rate
}
