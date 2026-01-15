use alloy_primitives::{Address, Bytes};
use alloy_sol_types::SolCall;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra::rpc::RpcClient;

#[derive(Debug, Clone)]
pub struct Call {
    pub target: Address,
    pub call_data: Bytes,
}

#[derive(Clone)]
pub struct MulticallClient {
    rpc: RpcClient,
    multicall_address: Address,
    max_calls_per_batch: usize,
}

impl MulticallClient {
    pub fn new(rpc: RpcClient, multicall_address: Address) -> Self {
        Self {
            rpc,
            multicall_address,
            max_calls_per_batch: 100, // 增加批量大小以减少 RPC 调用
        }
    }

    pub async fn aggregate(
        &self,
        calls: Vec<Call>,
    ) -> Result<Vec<std::result::Result<Bytes, CroLensError>>> {
        let mut out = Vec::with_capacity(calls.len());
        for chunk in calls.chunks(self.max_calls_per_batch) {
            let mut call3s = Vec::with_capacity(chunk.len());
            for call in chunk {
                call3s.push(abi::Call3 {
                    target: call.target,
                    allowFailure: true,
                    callData: call.call_data.clone(),
                });
            }

            let data = abi::aggregate3Call { calls: call3s }.abi_encode();
            let response = self
                .rpc
                .eth_call(self.multicall_address, Bytes::from(data))
                .await?;
            let decoded = abi::aggregate3Call::abi_decode_returns(&response, true)
                .map_err(|err| CroLensError::RpcError(format!("Multicall decode failed: {err}")))?;

            for item in decoded.returnData {
                if item.success {
                    out.push(Ok(item.returnData));
                } else {
                    out.push(Err(CroLensError::RpcError(
                        "Multicall inner call failed".to_string(),
                    )));
                }
            }
        }

        Ok(out)
    }
}
