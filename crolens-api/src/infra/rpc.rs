use alloy_primitives::{Address, Bytes};
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
        self.enforce_circuit(method).await?;

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
                    self.on_rpc_success().await;
                    self.put_cache(&cache_key, &v).await;
                    return Ok(v);
                }
                Err(err) => {
                    self.on_rpc_failure().await;
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
}
