use alloy_primitives::{Address, U256};

use crate::error::Result;
use crate::infra::rpc::{InternalCall, RpcClient};

/// 交易模拟结果
/// 基础模式: 使用 eth_call + eth_estimateGas (所有 EVM RPC 支持)
/// 高级模式: 需要 debug_traceCall (大多数 RPC 不支持 Cronos)
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub success: bool,
    pub gas_used: Option<u64>,
    pub output: String,
    pub logs: Vec<SimulationLog>,
    pub internal_calls: Vec<InternalCall>,
    pub error_message: Option<String>,
    /// 是否为基础模式 (无日志/内部调用追踪)
    pub basic_mode: bool,
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

/// 模拟客户端 - 使用 eth_call + eth_estimateGas 实现基础模拟
/// 注意: Tenderly 已停止支持 Cronos，改用标准 RPC 方法
#[derive(Clone)]
pub struct SimulationClient {
    rpc: RpcClient,
}

impl SimulationClient {
    /// 从 RpcClient 创建模拟客户端
    pub fn new(rpc: RpcClient) -> Self {
        Self { rpc }
    }

    /// 模拟交易执行
    /// 使用 eth_call + eth_estimateGas 提供:
    /// - ✅ 交易成功/失败预测
    /// - ✅ Gas 估算
    /// - ✅ 合约返回值
    /// - ❌ 事件日志 (需要 debug_traceCall)
    /// - ❌ 内部调用追踪 (需要 debug_traceCall)
    pub async fn simulate(
        &self,
        from: Address,
        to: Address,
        input: &str,
        value: U256,
        _gas: Option<u64>, // 保留参数以保持 API 兼容
    ) -> Result<SimulationResult> {
        let result = self.rpc.simulate_basic(from, to, input, value).await?;

        Ok(SimulationResult {
            success: result.success,
            gas_used: result.gas_used,
            output: result.output,
            logs: vec![], // 基础模式无法获取日志
            internal_calls: vec![], // 基础模式无法获取内部调用
            error_message: result.error_message,
            basic_mode: true,
        })
    }
}

// 保留旧的类型别名以兼容现有代码
pub type TenderlyClient = SimulationClient;
