pub mod config;
pub mod db;
pub mod logging;
pub mod multicall;
pub mod price;
pub mod rpc;
pub mod structured_log;
pub mod tenderly;
pub mod token;
pub mod x402;

use worker::kv::KvStore;
use worker::{D1Database, Env};

use crate::error::{CroLensError, Result};
use crate::types;

pub struct Services {
    pub trace_id: String,
    pub start_ms: i64,
    rpc: Option<rpc::RpcClient>,
    multicall: Option<multicall::MulticallClient>,
    tenderly: Option<tenderly::TenderlyClient>,
    pub db: D1Database,
    pub kv: KvStore,
}

impl Services {
    pub fn new(env: &Env, trace_id: &str, start_ms: i64) -> Result<Self> {
        let db = env
            .d1("DB")
            .map_err(|err| CroLensError::DbError(err.to_string()))?;
        let kv = env
            .kv("KV")
            .map_err(|err| CroLensError::KvError(err.to_string()))?;
        let multicall_address = env
            .var("MULTICALL3_ADDRESS")
            .map(|v| v.to_string())
            .ok()
            .and_then(|v| types::parse_address(&v).ok())
            .unwrap_or_else(|| {
                types::parse_address("0xcA11bde05977b3631167028862bE2a173976CA11")
                    .unwrap_or_default()
            });

        let rpc = rpc::RpcClient::try_new(env, Some(kv.clone()));
        let multicall = rpc
            .as_ref()
            .map(|client| multicall::MulticallClient::new(client.clone(), multicall_address));
        // 模拟客户端: 使用 eth_call + eth_estimateGas (Tenderly 已停止支持 Cronos)
        let tenderly = rpc.as_ref().map(|client| tenderly::SimulationClient::new(client.clone()));
        Ok(Self {
            trace_id: trace_id.to_string(),
            start_ms,
            rpc,
            multicall,
            tenderly,
            db,
            kv,
        })
    }

    pub fn rpc(&self) -> Result<&rpc::RpcClient> {
        self.rpc
            .as_ref()
            .ok_or_else(|| CroLensError::RpcError("Missing env var: BLOCKPI_RPC_URL".to_string()))
    }

    pub fn multicall(&self) -> Result<&multicall::MulticallClient> {
        self.multicall
            .as_ref()
            .ok_or_else(|| CroLensError::RpcError("Missing env var: BLOCKPI_RPC_URL".to_string()))
    }

    pub fn tenderly(&self) -> Option<&tenderly::TenderlyClient> {
        self.tenderly.as_ref()
    }

    pub fn meta(&self) -> serde_json::Value {
        let now = types::now_ms();
        serde_json::json!({
            "trace_id": self.trace_id,
            "timestamp": now,
            "latency_ms": now.saturating_sub(self.start_ms),
            "cached": false,
        })
    }
}
