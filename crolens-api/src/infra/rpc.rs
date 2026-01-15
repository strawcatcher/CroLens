use alloy_primitives::{Address, Bytes, U256};
use futures_util::future::{select, Either, FutureExt};
use futures_util::pin_mut;
use serde_json::Value;
use std::time::Duration;
use worker::{console_warn, Delay, KvStore};
use worker::{Fetch, Headers, Method, Request, RequestInit};

use crate::error::{CroLensError, Result};
use crate::types;

const RPC_CACHE_PREFIX: &str = "rpc:cache:";
const RPC_CIRCUIT_OPEN_UNTIL_KEY: &str = "rpc:cb:open_until_ms";
const RPC_CIRCUIT_FAIL_COUNT_KEY: &str = "rpc:cb:fail_count";
const RPC_CIRCUIT_LAST_PROBE_KEY: &str = "rpc:cb:last_probe_ms";

const RPC_DEFAULT_TIMEOUT_MS: u64 = 10_000;
const RPC_DEFAULT_CACHE_TTL_SECS: u64 = 300;

const RPC_CIRCUIT_WINDOW_SECS: u64 = 300;
const RPC_CIRCUIT_OPEN_SECS: u64 = 300;
const RPC_CIRCUIT_FAIL_THRESHOLD: i64 = 10;
const RPC_CIRCUIT_PROBE_INTERVAL_MS: i64 = 60_000;

#[derive(Clone)]
pub struct RpcClient {
    url: String,
    max_retries: u8,
    timeout_ms: u64,
    cache_ttl_secs: u64,
    kv: Option<KvStore>,
}

impl RpcClient {
    pub fn try_new(env: &worker::Env, kv: Option<KvStore>) -> Option<Self> {
        let url = env.var("BLOCKPI_RPC_URL").ok()?.to_string();
        if url.trim().is_empty() {
            return None;
        }
        let max_retries = env
            .var("RPC_MAX_RETRIES")
            .ok()
            .and_then(|v| v.to_string().parse::<u8>().ok())
            .unwrap_or(3);
        let timeout_ms = env
            .var("RPC_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.to_string().parse::<u64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(RPC_DEFAULT_TIMEOUT_MS);
        let cache_ttl_secs = env
            .var("RPC_CACHE_TTL_SECS")
            .ok()
            .and_then(|v| v.to_string().parse::<u64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(RPC_DEFAULT_CACHE_TTL_SECS);

        Some(Self {
            url,
            max_retries,
            timeout_ms,
            cache_ttl_secs,
            kv,
        })
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        // 简化版：跳过 circuit breaker 检查以减少 KV 延迟
        // self.enforce_circuit(method).await?;

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let body = serde_json::to_string(&payload)
            .map_err(|err| CroLensError::RpcError(err.to_string()))?;
        let mut last_err: Option<CroLensError> = None;
        let cache_key = self.cache_key(method, &body);

        for _ in 0..self.max_retries {
            match self.send_with_timeout(&body).await {
                Ok(v) => {
                    // 跳过 on_rpc_success 的 KV 操作以减少延迟
                    // self.on_rpc_success().await;
                    // 缓存写入不等待结果
                    self.put_cache_fire_and_forget(&cache_key, &v);
                    return Ok(v);
                }
                Err(err) => {
                    // 跳过 on_rpc_failure 的 KV 操作以减少延迟
                    // self.on_rpc_failure().await;
                    last_err = Some(err);

                    if let Some(cached) = self.get_cache(&cache_key).await {
                        console_warn!(
                            "[WARN] RPC failed for {}, returning cached response",
                            method
                        );
                        return Ok(cached);
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| CroLensError::RpcError("RPC retries exhausted".to_string())))
    }

    async fn send_with_timeout(&self, body: &str) -> Result<Value> {
        let fut = self.send(body).fuse();
        let timeout = Delay::from(Duration::from_millis(self.timeout_ms)).fuse();
        pin_mut!(fut, timeout);
        match select(fut, timeout).await {
            Either::Left((out, _)) => out,
            Either::Right((_elapsed, _)) => Err(CroLensError::RpcError(format!(
                "RPC timeout after {}ms",
                self.timeout_ms
            ))),
        }
    }

    async fn send(&self, body: &str) -> Result<Value> {
        let headers = Headers::new();
        headers
            .set("Content-Type", "application/json")
            .map_err(|err| CroLensError::RpcError(err.to_string()))?;

        let mut init = RequestInit::new();
        init.with_method(Method::Post);
        init.with_headers(headers);
        init.with_body(Some(body.into()));

        let request = Request::new_with_init(&self.url, &init)
            .map_err(|err| CroLensError::RpcError(err.to_string()))?;
        let mut resp = Fetch::Request(request)
            .send()
            .await
            .map_err(|err| CroLensError::RpcError(err.to_string()))?;
        let value: Value = resp
            .json()
            .await
            .map_err(|err| CroLensError::RpcError(err.to_string()))?;

        if let Some(err) = value.get("error") {
            let message = err
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown RPC error");
            return Err(CroLensError::RpcError(message.to_string()));
        }

        value
            .get("result")
            .cloned()
            .ok_or_else(|| CroLensError::RpcError("Missing RPC result".to_string()))
    }

    fn cache_key(&self, method: &str, body: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        body.hash(&mut hasher);
        let hash = hasher.finish();
        format!("{RPC_CACHE_PREFIX}{method}:{hash:016x}")
    }

    async fn get_cache(&self, key: &str) -> Option<Value> {
        let kv = self.kv.as_ref()?;
        let raw = kv.get(key).text().await.ok().flatten()?;
        serde_json::from_str::<Value>(&raw).ok()
    }

    async fn put_cache(&self, key: &str, value: &Value) {
        let kv = match self.kv.as_ref() {
            Some(v) => v,
            None => return,
        };

        let Ok(raw) = serde_json::to_string(value) else {
            return;
        };

        let Ok(put) = kv.put(key, raw) else {
            return;
        };
        let _ = put.expiration_ttl(self.cache_ttl_secs).execute().await;
    }

    fn put_cache_fire_and_forget(&self, key: &str, value: &Value) {
        let kv = match self.kv.as_ref() {
            Some(v) => v,
            None => return,
        };

        let Ok(raw) = serde_json::to_string(value) else {
            return;
        };

        let key = key.to_string();
        let ttl = self.cache_ttl_secs;
        let kv = kv.clone();
        // Fire and forget - 不等待结果
        worker::wasm_bindgen_futures::spawn_local(async move {
            if let Ok(put) = kv.put(&key, raw) {
                let _ = put.expiration_ttl(ttl).execute().await;
            }
        });
    }

    async fn enforce_circuit(&self, method: &str) -> Result<()> {
        let kv = match self.kv.as_ref() {
            Some(v) => v,
            None => return Ok(()),
        };

        let now = types::now_ms();
        let open_until_ms = kv
            .get(RPC_CIRCUIT_OPEN_UNTIL_KEY)
            .text()
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok());

        let Some(open_until_ms) = open_until_ms else {
            return Ok(());
        };

        if now >= open_until_ms {
            let _ = kv.delete(RPC_CIRCUIT_OPEN_UNTIL_KEY).await;
            let _ = kv.delete(RPC_CIRCUIT_LAST_PROBE_KEY).await;
            return Ok(());
        }

        let last_probe_ms = kv
            .get(RPC_CIRCUIT_LAST_PROBE_KEY)
            .text()
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);

        if now.saturating_sub(last_probe_ms) >= RPC_CIRCUIT_PROBE_INTERVAL_MS {
            if let Ok(put) = kv.put(RPC_CIRCUIT_LAST_PROBE_KEY, now.to_string()) {
                let _ = put.expiration_ttl(RPC_CIRCUIT_OPEN_SECS).execute().await;
            }
            return Ok(());
        }

        let retry_after_secs = ((open_until_ms.saturating_sub(now) / 1000).max(1) as u32)
            .min(RPC_CIRCUIT_OPEN_SECS as u32);

        Err(CroLensError::service_unavailable(
            format!("RPC circuit open for {}", method),
            Some(retry_after_secs),
        ))
    }

    async fn on_rpc_success(&self) {
        let kv = match self.kv.as_ref() {
            Some(v) => v,
            None => return,
        };
        let _ = kv.delete(RPC_CIRCUIT_FAIL_COUNT_KEY).await;
        let _ = kv.delete(RPC_CIRCUIT_OPEN_UNTIL_KEY).await;
        let _ = kv.delete(RPC_CIRCUIT_LAST_PROBE_KEY).await;
    }

    async fn on_rpc_failure(&self) {
        let kv = match self.kv.as_ref() {
            Some(v) => v,
            None => return,
        };

        let current = kv
            .get(RPC_CIRCUIT_FAIL_COUNT_KEY)
            .text()
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);
        let next = current.saturating_add(1);

        if let Ok(put) = kv.put(RPC_CIRCUIT_FAIL_COUNT_KEY, next.to_string()) {
            let _ = put.expiration_ttl(RPC_CIRCUIT_WINDOW_SECS).execute().await;
        }

        if next < RPC_CIRCUIT_FAIL_THRESHOLD {
            return;
        }

        let now = types::now_ms();
        let open_until_ms = now.saturating_add((RPC_CIRCUIT_OPEN_SECS as i64) * 1000);
        if let Ok(put) = kv.put(RPC_CIRCUIT_OPEN_UNTIL_KEY, open_until_ms.to_string()) {
            let _ = put.expiration_ttl(RPC_CIRCUIT_OPEN_SECS).execute().await;
        }
        if let Ok(put) = kv.put(RPC_CIRCUIT_LAST_PROBE_KEY, now.to_string()) {
            let _ = put.expiration_ttl(RPC_CIRCUIT_OPEN_SECS).execute().await;
        }
    }

    pub async fn eth_call(&self, to: Address, data: Bytes) -> Result<Vec<u8>> {
        let to_hex = to.to_string();
        let data_hex = types::bytes_to_hex0x(&data);
        let result = self
            .call(
                "eth_call",
                serde_json::json!([{ "to": to_hex, "data": data_hex }, "latest"]),
            )
            .await?;
        let output = result
            .as_str()
            .ok_or_else(|| CroLensError::RpcError("eth_call result is not a string".to_string()))?;
        types::hex0x_to_bytes(output)
    }

    pub async fn eth_get_transaction_by_hash(&self, tx_hash: &str) -> Result<Value> {
        self.call("eth_getTransactionByHash", serde_json::json!([tx_hash]))
            .await
    }

    pub async fn eth_get_transaction_receipt(&self, tx_hash: &str) -> Result<Value> {
        self.call("eth_getTransactionReceipt", serde_json::json!([tx_hash]))
            .await
    }

    /// 使用 debug_traceCall 模拟交易执行
    /// 提供: 成功/失败预测, Gas 估算, 内部调用追踪, 状态变化检测
    pub async fn debug_trace_call(
        &self,
        from: Address,
        to: Address,
        data: &str,
        value: U256,
        gas: Option<u64>,
    ) -> Result<DebugTraceResult> {
        // 构建交易对象，包含 gas 限制
        let gas_limit = gas.unwrap_or(5_000_000); // 默认 5M gas
        let tx_obj = serde_json::json!({
            "from": from.to_string(),
            "to": to.to_string(),
            "data": data,
            "value": format!("0x{:x}", value),
            "gas": format!("0x{:x}", gas_limit),
        });

        // 使用 callTracer 获取内部调用和日志
        let tracer_config = serde_json::json!({
            "tracer": "callTracer",
            "tracerConfig": {
                "withLog": true
            }
        });

        let result = self
            .call("debug_traceCall", serde_json::json!([tx_obj, "latest", tracer_config]))
            .await?;

        // 解析 callTracer 结果
        let output = result.get("output").and_then(|v| v.as_str()).unwrap_or("0x");
        let gas_used = result
            .get("gasUsed")
            .and_then(|v| v.as_str())
            .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok());
        let error = result.get("error").and_then(|v| v.as_str()).map(|v| v.to_string());
        let revert_reason = result
            .get("revertReason")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());

        // 提取日志
        let logs = extract_logs_from_trace(&result);

        // 提取内部调用
        let internal_calls = extract_internal_calls(&result);

        let success = error.is_none() && revert_reason.is_none();
        let error_message = error.or(revert_reason);

        Ok(DebugTraceResult {
            success,
            gas_used,
            output: output.to_string(),
            logs,
            internal_calls,
            error_message,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DebugTraceResult {
    pub success: bool,
    pub gas_used: Option<u64>,
    pub output: String,
    pub logs: Vec<DebugTraceLog>,
    pub internal_calls: Vec<InternalCall>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DebugTraceLog {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
}

/// 内部调用信息
#[derive(Debug, Clone)]
pub struct InternalCall {
    pub call_type: String, // CALL, STATICCALL, DELEGATECALL, CREATE, etc.
    pub from: String,
    pub to: String,
    pub value: String,
    pub gas_used: Option<u64>,
    pub input: String,
    pub output: String,
    pub error: Option<String>,
}

/// 从 callTracer 结果中递归提取所有日志
fn extract_logs_from_trace(trace: &Value) -> Vec<DebugTraceLog> {
    let mut logs = Vec::new();
    extract_logs_recursive(trace, &mut logs);
    logs
}

fn extract_logs_recursive(trace: &Value, logs: &mut Vec<DebugTraceLog>) {
    // 提取当前层的日志
    if let Some(trace_logs) = trace.get("logs").and_then(|v| v.as_array()) {
        for log in trace_logs {
            let address = log
                .get("address")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_lowercase();
            let topics = log
                .get("topics")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_lowercase()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let data = log
                .get("data")
                .and_then(|v| v.as_str())
                .unwrap_or("0x")
                .to_lowercase();

            logs.push(DebugTraceLog {
                address,
                topics,
                data,
            });
        }
    }

    // 递归处理子调用
    if let Some(calls) = trace.get("calls").and_then(|v| v.as_array()) {
        for call in calls {
            extract_logs_recursive(call, logs);
        }
    }
}

/// 从 callTracer 结果中提取内部调用
fn extract_internal_calls(trace: &Value) -> Vec<InternalCall> {
    let mut calls = Vec::new();
    extract_calls_recursive(trace, &mut calls, true);
    calls
}

fn extract_calls_recursive(trace: &Value, calls: &mut Vec<InternalCall>, is_root: bool) {
    // 跳过根调用，只提取内部调用
    if !is_root {
        let call_type = trace
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("CALL")
            .to_uppercase();
        let from = trace
            .get("from")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_lowercase();
        let to = trace
            .get("to")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_lowercase();
        let value = trace
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0")
            .to_string();
        let gas_used = trace
            .get("gasUsed")
            .and_then(|v| v.as_str())
            .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok());
        let input = trace
            .get("input")
            .and_then(|v| v.as_str())
            .unwrap_or("0x")
            .to_string();
        let output = trace
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("0x")
            .to_string();
        let error = trace
            .get("error")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());

        calls.push(InternalCall {
            call_type,
            from,
            to,
            value,
            gas_used,
            input,
            output,
            error,
        });
    }

    // 递归处理子调用
    if let Some(sub_calls) = trace.get("calls").and_then(|v| v.as_array()) {
        for call in sub_calls {
            extract_calls_recursive(call, calls, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ============ extract_logs_from_trace tests ============

    #[test]
    fn test_extract_logs_single_level() {
        let trace = json!({
            "logs": [
                {
                    "address": "0xC21223249CA28397B4B6541dFFaEcC539bfF0c59",
                    "topics": [
                        "0xDDF252AD1BE2C89B69C2B068FC378DAA952BA7F163C4A11628F55A4DF523B3EF",
                        "0x0000000000000000000000001111111111111111111111111111111111111111"
                    ],
                    "data": "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000"
                }
            ]
        });

        let logs = extract_logs_from_trace(&trace);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].address, "0xc21223249ca28397b4b6541dffaecc539bff0c59");
        assert_eq!(logs[0].topics.len(), 2);
        assert_eq!(
            logs[0].topics[0],
            "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
        );
    }

    #[test]
    fn test_extract_logs_nested_calls() {
        let trace = json!({
            "logs": [
                {
                    "address": "0x1111111111111111111111111111111111111111",
                    "topics": ["0xaaa"],
                    "data": "0x111"
                }
            ],
            "calls": [
                {
                    "logs": [
                        {
                            "address": "0x2222222222222222222222222222222222222222",
                            "topics": ["0xbbb"],
                            "data": "0x222"
                        }
                    ],
                    "calls": [
                        {
                            "logs": [
                                {
                                    "address": "0x3333333333333333333333333333333333333333",
                                    "topics": ["0xccc"],
                                    "data": "0x333"
                                }
                            ]
                        }
                    ]
                }
            ]
        });

        let logs = extract_logs_from_trace(&trace);
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].address, "0x1111111111111111111111111111111111111111");
        assert_eq!(logs[1].address, "0x2222222222222222222222222222222222222222");
        assert_eq!(logs[2].address, "0x3333333333333333333333333333333333333333");
    }

    #[test]
    fn test_extract_logs_empty() {
        let trace = json!({});
        let logs = extract_logs_from_trace(&trace);
        assert!(logs.is_empty());
    }

    #[test]
    fn test_extract_logs_no_logs_but_calls() {
        let trace = json!({
            "calls": [
                {
                    "logs": [
                        {
                            "address": "0xABCDABCDABCDABCDABCDABCDABCDABCDABCDABCD",
                            "topics": [],
                            "data": "0x"
                        }
                    ]
                }
            ]
        });

        let logs = extract_logs_from_trace(&trace);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].address, "0xabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd");
    }

    #[test]
    fn test_extract_logs_missing_fields() {
        let trace = json!({
            "logs": [
                {
                    // missing address - tests default value handling
                    "topics": ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"],
                    "data": "0x0000000000000000000000000000000000000000000000000000000000000456"
                },
                {
                    "address": "0x4444444444444444444444444444444444444444",
                    // missing topics - tests default value handling
                    "data": "0x0000000000000000000000000000000000000000000000000000000000000789"
                }
            ]
        });

        let logs = extract_logs_from_trace(&trace);
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].address, ""); // default for missing
        assert!(logs[1].topics.is_empty()); // default for missing
    }

    // ============ extract_internal_calls tests ============

    #[test]
    fn test_extract_internal_calls_simple() {
        let trace = json!({
            "type": "CALL",
            "from": "0x1111111111111111111111111111111111111111",
            "to": "0x2222222222222222222222222222222222222222",
            "calls": [
                {
                    "type": "STATICCALL",
                    "from": "0x2222222222222222222222222222222222222222",
                    "to": "0x3333333333333333333333333333333333333333",
                    "value": "0x0",
                    "gasUsed": "0x5208",
                    "input": "0xabcd",
                    "output": "0x1234"
                }
            ]
        });

        let calls = extract_internal_calls(&trace);
        assert_eq!(calls.len(), 1); // Root call is skipped
        assert_eq!(calls[0].call_type, "STATICCALL");
        assert_eq!(calls[0].from, "0x2222222222222222222222222222222222222222");
        assert_eq!(calls[0].to, "0x3333333333333333333333333333333333333333");
        assert_eq!(calls[0].gas_used, Some(21000)); // 0x5208 = 21000
    }

    #[test]
    fn test_extract_internal_calls_nested() {
        // User wallet -> Router -> Pair -> Token contracts
        let trace = json!({
            "type": "CALL",
            "from": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23", // user wallet
            "to": "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae",   // VVS Router
            "calls": [
                {
                    "type": "CALL",
                    "from": "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae", // Router
                    "to": "0xbF62c67eA509E86F07c8c69d0286C0636C50270b",   // CRO-USDC Pair
                    "value": "0x1",
                    "calls": [
                        {
                            "type": "DELEGATECALL",
                            "from": "0xbF62c67eA509E86F07c8c69d0286C0636C50270b", // Pair
                            "to": "0xc21223249CA28397B4B6541dFFaEcC539bfF0c59",   // USDC Token
                            "value": "0x0"
                        }
                    ]
                },
                {
                    "type": "STATICCALL",
                    "from": "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae", // Router
                    "to": "0xe6DaD9a92E574c4AD54e95A6f9B3b31a66C0dc9e"   // Price Oracle
                }
            ]
        });

        let calls = extract_internal_calls(&trace);
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].call_type, "CALL");
        // extract_calls_recursive lowercases addresses
        assert_eq!(calls[0].to, "0xbf62c67ea509e86f07c8c69d0286c0636c50270b");
        assert_eq!(calls[1].call_type, "DELEGATECALL");
        assert_eq!(calls[1].to, "0xc21223249ca28397b4b6541dffaecc539bff0c59");
        assert_eq!(calls[2].call_type, "STATICCALL");
        assert_eq!(calls[2].to, "0xe6dad9a92e574c4ad54e95a6f9b3b31a66c0dc9e");
    }

    #[test]
    fn test_extract_internal_calls_empty() {
        // Simple transfer with no internal calls
        let trace = json!({
            "type": "CALL",
            "from": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23",
            "to": "0xc21223249CA28397B4B6541dFFaEcC539bfF0c59"
        });

        let calls = extract_internal_calls(&trace);
        assert!(calls.is_empty()); // No internal calls, just root
    }

    #[test]
    fn test_extract_internal_calls_with_error() {
        // Contract call that fails during internal execution
        let trace = json!({
            "type": "CALL",
            "from": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23",
            "to": "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae",
            "calls": [
                {
                    "type": "CALL",
                    "from": "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae",
                    "to": "0xDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF",
                    "error": "execution reverted"
                }
            ]
        });

        let calls = extract_internal_calls(&trace);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].error, Some("execution reverted".to_string()));
    }

    #[test]
    fn test_extract_internal_calls_gas_parsing() {
        let trace = json!({
            "calls": [
                {
                    "type": "CALL",
                    "from": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23",
                    "to": "0xc21223249CA28397B4B6541dFFaEcC539bfF0c59",
                    "gasUsed": "0x1234" // 4660 in decimal
                },
                {
                    "type": "CALL",
                    "from": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23",
                    "to": "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae",
                    "gasUsed": "invalid" // invalid hex - tests error handling
                }
            ]
        });

        let calls = extract_internal_calls(&trace);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].gas_used, Some(4660));
        assert_eq!(calls[1].gas_used, None); // invalid parsing returns None
    }

    // ============ DebugTraceResult parsing tests ============

    #[test]
    fn test_debug_trace_result_success_detection() {
        // Test the logic used in debug_trace_call for success detection
        let result_success = json!({
            "output": "0x1234",
            "gasUsed": "0x5208"
        });

        let error = result_success.get("error").and_then(|v| v.as_str()).map(|v| v.to_string());
        let revert_reason = result_success
            .get("revertReason")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let success = error.is_none() && revert_reason.is_none();
        assert!(success);

        let result_error = json!({
            "output": "0x",
            "error": "execution reverted"
        });

        let error = result_error.get("error").and_then(|v| v.as_str()).map(|v| v.to_string());
        let revert_reason = result_error
            .get("revertReason")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let success = error.is_none() && revert_reason.is_none();
        assert!(!success);
        assert_eq!(error, Some("execution reverted".to_string()));
    }

    #[test]
    fn test_debug_trace_gas_parsing() {
        let result = json!({
            "gasUsed": "0x5208" // 21000
        });

        let gas_used = result
            .get("gasUsed")
            .and_then(|v| v.as_str())
            .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok());

        assert_eq!(gas_used, Some(21000));
    }

    #[test]
    fn test_debug_trace_output_parsing() {
        let result = json!({
            "output": "0x0000000000000000000000000000000000000000000000000000000000000001"
        });

        let output = result.get("output").and_then(|v| v.as_str()).unwrap_or("0x");
        assert_eq!(
            output,
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        );

        // Test missing output
        let result_no_output = json!({});
        let output = result_no_output
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("0x");
        assert_eq!(output, "0x");
    }

    // ============ Integration-like tests with realistic data ============

    #[test]
    fn test_realistic_erc20_transfer_trace() {
        // Simulates a real ERC20 transfer trace from debug_traceCall
        let trace = json!({
            "type": "CALL",
            "from": "0x5c7f8a570d578ed84e63fdfa7b1ee72deae1ae23",
            "to": "0xc21223249ca28397b4b6541dffaecc539bff0c59",
            "value": "0x0",
            "gas": "0x7a120",
            "gasUsed": "0xcf08",
            "input": "0xa9059cbb0000000000000000000000001234567890123456789012345678901234567890000000000000000000000000000000000000000000000000000000000000000a",
            "output": "0x0000000000000000000000000000000000000000000000000000000000000001",
            "logs": [
                {
                    "address": "0xc21223249ca28397b4b6541dffaecc539bff0c59",
                    "topics": [
                        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                        "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23",
                        "0x0000000000000000000000001234567890123456789012345678901234567890"
                    ],
                    "data": "0x000000000000000000000000000000000000000000000000000000000000000a"
                }
            ]
        });

        let logs = extract_logs_from_trace(&trace);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].address, "0xc21223249ca28397b4b6541dffaecc539bff0c59");
        assert_eq!(logs[0].topics.len(), 3);

        let calls = extract_internal_calls(&trace);
        assert!(calls.is_empty()); // Simple transfer has no internal calls
    }

    #[test]
    fn test_realistic_swap_trace() {
        // Simulates a DEX swap with multiple internal calls
        let trace = json!({
            "type": "CALL",
            "from": "0xuser",
            "to": "0xrouter",
            "calls": [
                {
                    "type": "CALL",
                    "from": "0xrouter",
                    "to": "0xpair",
                    "value": "0x0",
                    "gasUsed": "0x1234",
                    "logs": [
                        {
                            "address": "0xpair",
                            "topics": [
                                "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"
                            ],
                            "data": "0x"
                        }
                    ],
                    "calls": [
                        {
                            "type": "CALL",
                            "from": "0xpair",
                            "to": "0xtoken0",
                            "value": "0x0",
                            "gasUsed": "0x5678",
                            "logs": [
                                {
                                    "address": "0xtoken0",
                                    "topics": ["0xddf252ad"],
                                    "data": "0x1234"
                                }
                            ]
                        },
                        {
                            "type": "CALL",
                            "from": "0xpair",
                            "to": "0xtoken1",
                            "value": "0x0",
                            "gasUsed": "0x9abc"
                        }
                    ]
                }
            ]
        });

        let logs = extract_logs_from_trace(&trace);
        assert_eq!(logs.len(), 2); // Swap event + Transfer event

        let calls = extract_internal_calls(&trace);
        assert_eq!(calls.len(), 3); // router->pair, pair->token0, pair->token1
        assert_eq!(calls[0].to, "0xpair");
        assert_eq!(calls[1].to, "0xtoken0");
        assert_eq!(calls[2].to, "0xtoken1");
    }
}
