use alloy_primitives::{Address, U256};
use worker::console_log;

use crate::error::Result;
use crate::infra::rpc::{InternalCall, RpcClient};
use crate::types;

/// 模拟客户端，使用 RPC debug_traceCall (BlockPi 支持 Cronos)
#[derive(Clone)]
pub struct SimulationClient {
    rpc: RpcClient,
}

#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub success: bool,
    pub gas_used: Option<u64>,
    pub output: String,
    pub logs: Vec<SimulationLog>,
    pub internal_calls: Vec<InternalCall>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SimulationLog {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
}

// 向后兼容的类型别名
pub type TenderlySimulation = SimulationResult;
pub type TenderlyLog = SimulationLog;

impl SimulationClient {
    pub fn try_new(_env: &worker::Env, rpc: RpcClient) -> Self {
        Self { rpc }
    }

    /// 模拟交易执行
    /// 使用 BlockPi debug_traceCall 提供:
    /// - 交易成功/失败预测
    /// - Gas 估算
    /// - 内部调用追踪
    /// - 状态变化检测 (通过 logs)
    pub async fn simulate(
        &self,
        from: Address,
        to: Address,
        input: &str,
        value: U256,
        gas: Option<u64>,
    ) -> Result<SimulationResult> {
        let trace_result = self
            .rpc
            .debug_trace_call(from, to, input, value, gas)
            .await?;

        console_log!(
            "[INFO] Simulation via debug_traceCall: success={}, gas_used={:?}, logs={}, calls={}",
            trace_result.success,
            trace_result.gas_used,
            trace_result.logs.len(),
            trace_result.internal_calls.len()
        );

        let logs = trace_result
            .logs
            .into_iter()
            .map(|log| SimulationLog {
                address: normalize_address(&log.address),
                topics: log.topics.into_iter().map(|t| normalize_hex(&t)).collect(),
                data: normalize_hex(&log.data),
            })
            .collect();

        Ok(SimulationResult {
            success: trace_result.success,
            gas_used: trace_result.gas_used,
            output: trace_result.output,
            logs,
            internal_calls: trace_result.internal_calls,
            error_message: trace_result.error_message,
        })
    }
}

// 保持向后兼容的类型别名
pub type TenderlyClient = SimulationClient;

fn normalize_hex(value: &str) -> String {
    if value.trim().starts_with("0x") {
        value.trim().to_lowercase()
    } else {
        format!("0x{}", value.trim().to_lowercase())
    }
}

fn normalize_address(value: &str) -> String {
    match types::parse_address(value) {
        Ok(addr) => addr.to_string(),
        Err(_) => value.to_string(),
    }
}
