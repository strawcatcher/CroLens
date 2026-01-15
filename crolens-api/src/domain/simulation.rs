use alloy_primitives::U256;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::infra::rpc::InternalCall;
use crate::types;

// Cronos gas price: ~5000 gwei (baseFee), 常规交易约 5000-10000 gwei
const CRONOS_GAS_PRICE_GWEI: u64 = 5000;

#[derive(Debug, Deserialize)]
struct SimulateArgs {
    from: String,
    to: String,
    data: String,
    value: String,
    #[serde(default)]
    gas: Option<u64>,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn simulate_transaction(services: &infra::Services, args: Value) -> Result<Value> {
    let input: SimulateArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let from = types::parse_address(&input.from)?;
    let to = types::parse_address(&input.to)?;
    if !input.data.trim().starts_with("0x") {
        return Err(CroLensError::invalid_params(
            "data must be 0x-prefixed hex".to_string(),
        ));
    }
    let _data_bytes = types::hex0x_to_bytes(&input.data)?;

    let value = if input.value.trim().starts_with("0x") {
        types::parse_u256_hex(&input.value)?
    } else {
        types::parse_u256_dec(&input.value)?
    };

    let Some(simulator) = services.tenderly() else {
        if input.simple_mode {
            return Ok(serde_json::json!({
                "text": "Simulation not available (RPC not configured).",
                "meta": services.meta(),
            }));
        }

        return Ok(serde_json::json!({
            "success": false,
            "simulation_available": false,
            "meta": services.meta(),
        }));
    };

    let simulation = simulator
        .simulate(from, to, &input.data, value, input.gas)
        .await?;

    let gas_used = simulation.gas_used.unwrap_or(0);
    let gas_estimated = gas_used.to_string();

    // 计算 CRO 成本: gas_used * gas_price (gwei) / 1e9
    let estimated_cost_cro = if gas_used > 0 {
        let cost_wei = (gas_used as u128) * (CRONOS_GAS_PRICE_GWEI as u128) * 1_000_000_000;
        let cost_cro = (cost_wei as f64) / 1e18;
        Some(format!("{:.6}", cost_cro))
    } else {
        None
    };

    let state_changes = decode_state_changes(&simulation.logs);
    let internal_calls_json = format_internal_calls(&simulation.internal_calls);

    // 风险评估
    let (risk_level, warnings) = assess_risk(&simulation);

    if input.simple_mode {
        let text = if simulation.success {
            let cost_info = estimated_cost_cro
                .as_ref()
                .map(|c| format!(" | Cost: ~{c} CRO"))
                .unwrap_or_default();
            format!("Simulation success | Gas: {gas_estimated}{cost_info}")
        } else {
            format!(
                "Simulation failed | Reason: {}",
                simulation
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            )
        };
        return Ok(serde_json::json!({
            "text": text,
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "success": simulation.success,
        "gas_estimated": gas_estimated,
        "estimated_cost_cro": estimated_cost_cro,
        "return_data": simulation.output,
        "state_changes": state_changes,
        "internal_calls": internal_calls_json,
        "risk_assessment": { "level": risk_level, "warnings": warnings },
        "meta": services.meta(),
    }))
}

// 常见事件签名
const TRANSFER_TOPIC: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
const APPROVAL_TOPIC: &str = "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925";
const SWAP_TOPIC: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"; // UniswapV2
const SWAP_V3_TOPIC: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"; // UniswapV3
const DEPOSIT_TOPIC: &str = "0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"; // WETH Deposit
const WITHDRAWAL_TOPIC: &str = "0x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65"; // WETH Withdrawal

fn decode_state_changes(logs: &[infra::tenderly::SimulationLog]) -> Vec<Value> {
    let mut out = Vec::new();

    for log in logs {
        if log.topics.is_empty() {
            continue;
        }

        let topic0 = &log.topics[0];

        // ERC20 Transfer
        if topic0.eq_ignore_ascii_case(TRANSFER_TOPIC) && log.topics.len() >= 3 {
            let from = topic_to_address(&log.topics[1]);
            let to = topic_to_address(&log.topics[2]);
            let amount = types::parse_u256_hex(&log.data).unwrap_or(U256::ZERO);

            out.push(serde_json::json!({
                "type": "transfer",
                "description": "ERC20 Transfer",
                "from": from,
                "to": to,
                "amount": amount.to_string(),
                "token": log.address,
            }));
        }
        // ERC20 Approval
        else if topic0.eq_ignore_ascii_case(APPROVAL_TOPIC) && log.topics.len() >= 3 {
            let owner = topic_to_address(&log.topics[1]);
            let spender = topic_to_address(&log.topics[2]);
            let amount = types::parse_u256_hex(&log.data).unwrap_or(U256::ZERO);

            let is_unlimited = amount == U256::MAX;
            out.push(serde_json::json!({
                "type": "approval",
                "description": if is_unlimited { "Unlimited Approval" } else { "ERC20 Approval" },
                "owner": owner,
                "spender": spender,
                "amount": amount.to_string(),
                "unlimited": is_unlimited,
                "token": log.address,
            }));
        }
        // UniswapV2 Swap
        else if topic0.eq_ignore_ascii_case(SWAP_TOPIC) && log.topics.len() >= 3 {
            let sender = topic_to_address(&log.topics[1]);
            let recipient = topic_to_address(&log.topics[2]);

            // data: amount0In, amount1In, amount0Out, amount1Out (each 32 bytes)
            let data = log.data.trim_start_matches("0x");
            if data.len() >= 256 {
                let amount0_in = parse_u256_from_hex_slice(data, 0);
                let amount1_in = parse_u256_from_hex_slice(data, 64);
                let amount0_out = parse_u256_from_hex_slice(data, 128);
                let amount1_out = parse_u256_from_hex_slice(data, 192);

                out.push(serde_json::json!({
                    "type": "swap",
                    "description": "DEX Swap (V2)",
                    "sender": sender,
                    "recipient": recipient,
                    "amount0_in": amount0_in.to_string(),
                    "amount1_in": amount1_in.to_string(),
                    "amount0_out": amount0_out.to_string(),
                    "amount1_out": amount1_out.to_string(),
                    "pair": log.address,
                }));
            }
        }
        // UniswapV3 Swap
        else if topic0.eq_ignore_ascii_case(SWAP_V3_TOPIC) && log.topics.len() >= 3 {
            let sender = topic_to_address(&log.topics[1]);
            let recipient = topic_to_address(&log.topics[2]);

            out.push(serde_json::json!({
                "type": "swap",
                "description": "DEX Swap (V3)",
                "sender": sender,
                "recipient": recipient,
                "pool": log.address,
            }));
        }
        // WETH Deposit
        else if topic0.eq_ignore_ascii_case(DEPOSIT_TOPIC) && log.topics.len() >= 2 {
            let dst = topic_to_address(&log.topics[1]);
            let amount = types::parse_u256_hex(&log.data).unwrap_or(U256::ZERO);

            out.push(serde_json::json!({
                "type": "deposit",
                "description": "Wrapped Native Deposit",
                "to": dst,
                "amount": amount.to_string(),
                "token": log.address,
            }));
        }
        // WETH Withdrawal
        else if topic0.eq_ignore_ascii_case(WITHDRAWAL_TOPIC) && log.topics.len() >= 2 {
            let src = topic_to_address(&log.topics[1]);
            let amount = types::parse_u256_hex(&log.data).unwrap_or(U256::ZERO);

            out.push(serde_json::json!({
                "type": "withdrawal",
                "description": "Wrapped Native Withdrawal",
                "from": src,
                "amount": amount.to_string(),
                "token": log.address,
            }));
        }
    }

    out
}

fn parse_u256_from_hex_slice(data: &str, offset: usize) -> U256 {
    if data.len() < offset + 64 {
        return U256::ZERO;
    }
    let slice = &data[offset..offset + 64];
    types::parse_u256_hex(&format!("0x{slice}")).unwrap_or(U256::ZERO)
}

fn topic_to_address(topic: &str) -> String {
    let trimmed = topic.trim().trim_start_matches("0x");
    if trimmed.len() < 40 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    let addr_hex = &trimmed[trimmed.len() - 40..];
    format!("0x{addr_hex}")
}

fn format_internal_calls(calls: &[InternalCall]) -> Vec<Value> {
    calls
        .iter()
        .map(|call| {
            serde_json::json!({
                "type": call.call_type,
                "from": call.from,
                "to": call.to,
                "value": call.value,
                "gas_used": call.gas_used,
                "error": call.error,
            })
        })
        .collect()
}

/// 风险评估
fn assess_risk(simulation: &infra::tenderly::SimulationResult) -> (&'static str, Vec<String>) {
    let mut warnings = Vec::new();

    // 交易失败
    if !simulation.success {
        if let Some(err) = &simulation.error_message {
            warnings.push(err.clone());
        } else {
            warnings.push("Transaction reverted".to_string());
        }
        return ("high", warnings);
    }

    // 检查是否有无限授权
    for log in &simulation.logs {
        if !log.topics.is_empty() && log.topics[0].eq_ignore_ascii_case(APPROVAL_TOPIC) {
            let amount = types::parse_u256_hex(&log.data).unwrap_or(U256::ZERO);
            if amount == U256::MAX {
                warnings.push("Unlimited token approval detected".to_string());
            }
        }
    }

    // 检查内部调用是否有失败
    for call in &simulation.internal_calls {
        if call.error.is_some() {
            warnings.push(format!(
                "Internal call to {} failed",
                &call.to[..10.min(call.to.len())]
            ));
        }
    }

    let level = if warnings.is_empty() {
        "low"
    } else if warnings.iter().any(|w| w.contains("Unlimited")) {
        "medium"
    } else {
        "low"
    };

    (level, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::rpc::InternalCall;
    use crate::infra::tenderly::SimulationLog;

    // ============ decode_state_changes tests ============

    #[test]
    fn test_decode_transfer_event() {
        let logs = vec![SimulationLog {
            address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59".to_string(), // USDC
            topics: vec![
                TRANSFER_TOPIC.to_string(),
                "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23".to_string(), // from
                "0x0000000000000000000000001234567890123456789012345678901234567890".to_string(), // to
            ],
            data: "0x00000000000000000000000000000000000000000000000000000000000f4240".to_string(), // 1000000
        }];

        let changes = decode_state_changes(&logs);
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change["type"], "transfer");
        assert_eq!(change["description"], "ERC20 Transfer");
        assert_eq!(
            change["from"],
            "0x5c7f8a570d578ed84e63fdfa7b1ee72deae1ae23"
        );
        assert_eq!(
            change["to"],
            "0x1234567890123456789012345678901234567890"
        );
        assert_eq!(change["amount"], "1000000");
    }

    #[test]
    fn test_decode_approval_event() {
        let logs = vec![SimulationLog {
            address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59".to_string(),
            topics: vec![
                APPROVAL_TOPIC.to_string(),
                "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23".to_string(), // owner
                "0x000000000000000000000000145863eb42cf62847a6ca784e6416c1682b1b2ae".to_string(), // spender
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(), // 1e18
        }];

        let changes = decode_state_changes(&logs);
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change["type"], "approval");
        assert_eq!(change["description"], "ERC20 Approval");
        assert_eq!(change["unlimited"], false);
    }

    #[test]
    fn test_decode_unlimited_approval() {
        let logs = vec![SimulationLog {
            address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59".to_string(),
            topics: vec![
                APPROVAL_TOPIC.to_string(),
                "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23".to_string(),
                "0x000000000000000000000000145863eb42cf62847a6ca784e6416c1682b1b2ae".to_string(),
            ],
            // max uint256
            data: "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
        }];

        let changes = decode_state_changes(&logs);
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change["type"], "approval");
        assert_eq!(change["description"], "Unlimited Approval");
        assert_eq!(change["unlimited"], true);
    }

    #[test]
    fn test_decode_swap_v2_event() {
        // UniswapV2 Swap event data: amount0In, amount1In, amount0Out, amount1Out (each 32 bytes)
        let data = format!(
            "0x{}{}{}{}",
            "0000000000000000000000000000000000000000000000000de0b6b3a7640000", // amount0In = 1e18
            "0000000000000000000000000000000000000000000000000000000000000000", // amount1In = 0
            "0000000000000000000000000000000000000000000000000000000000000000", // amount0Out = 0
            "00000000000000000000000000000000000000000000000000000000000f4240"  // amount1Out = 1000000
        );

        let logs = vec![SimulationLog {
            address: "0xbf62c67eA509E86F07c8c69d0286C0636C50270b".to_string(), // CRO-USDC pair
            topics: vec![
                SWAP_TOPIC.to_string(),
                "0x000000000000000000000000145863eb42cf62847a6ca784e6416c1682b1b2ae".to_string(), // sender
                "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23".to_string(), // recipient
            ],
            data,
        }];

        let changes = decode_state_changes(&logs);
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change["type"], "swap");
        assert_eq!(change["description"], "DEX Swap (V2)");
        assert_eq!(change["amount0_in"], "1000000000000000000");
        assert_eq!(change["amount1_in"], "0");
        assert_eq!(change["amount0_out"], "0");
        assert_eq!(change["amount1_out"], "1000000");
    }

    #[test]
    fn test_decode_deposit_event() {
        let logs = vec![SimulationLog {
            address: "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(), // WCRO
            topics: vec![
                DEPOSIT_TOPIC.to_string(),
                "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
        }];

        let changes = decode_state_changes(&logs);
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change["type"], "deposit");
        assert_eq!(change["description"], "Wrapped Native Deposit");
    }

    #[test]
    fn test_decode_withdrawal_event() {
        let logs = vec![SimulationLog {
            address: "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(),
            topics: vec![
                WITHDRAWAL_TOPIC.to_string(),
                "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
        }];

        let changes = decode_state_changes(&logs);
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change["type"], "withdrawal");
        assert_eq!(change["description"], "Wrapped Native Withdrawal");
    }

    #[test]
    fn test_decode_empty_logs() {
        let logs: Vec<SimulationLog> = vec![];
        let changes = decode_state_changes(&logs);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_decode_unknown_event() {
        let logs = vec![SimulationLog {
            address: "0x1234567890123456789012345678901234567890".to_string(),
            topics: vec![
                "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            ],
            data: "0x1234".to_string(),
        }];

        let changes = decode_state_changes(&logs);
        assert!(changes.is_empty()); // Unknown events are skipped
    }

    #[test]
    fn test_decode_event_with_empty_topics() {
        let logs = vec![SimulationLog {
            address: "0x1234567890123456789012345678901234567890".to_string(),
            topics: vec![],
            data: "0x1234".to_string(),
        }];

        let changes = decode_state_changes(&logs);
        assert!(changes.is_empty());
    }

    // ============ topic_to_address tests ============

    #[test]
    fn test_topic_to_address_normal() {
        let topic = "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23";
        let addr = topic_to_address(topic);
        assert_eq!(addr, "0x5c7f8a570d578ed84e63fdfa7b1ee72deae1ae23");
    }

    #[test]
    fn test_topic_to_address_short() {
        let topic = "0x123";
        let addr = topic_to_address(topic);
        assert_eq!(addr, "0x0000000000000000000000000000000000000000");
    }

    // ============ assess_risk tests ============

    #[test]
    fn test_assess_risk_success_no_warnings() {
        let simulation = infra::tenderly::SimulationResult {
            success: true,
            gas_used: Some(50000),
            output: "0x".to_string(),
            logs: vec![],
            internal_calls: vec![],
            error_message: None,
        };

        let (level, warnings) = assess_risk(&simulation);
        assert_eq!(level, "low");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_assess_risk_failed_with_error() {
        let simulation = infra::tenderly::SimulationResult {
            success: false,
            gas_used: None,
            output: "0x".to_string(),
            logs: vec![],
            internal_calls: vec![],
            error_message: Some("execution reverted".to_string()),
        };

        let (level, warnings) = assess_risk(&simulation);
        assert_eq!(level, "high");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("execution reverted"));
    }

    #[test]
    fn test_assess_risk_failed_no_message() {
        let simulation = infra::tenderly::SimulationResult {
            success: false,
            gas_used: None,
            output: "0x".to_string(),
            logs: vec![],
            internal_calls: vec![],
            error_message: None,
        };

        let (level, warnings) = assess_risk(&simulation);
        assert_eq!(level, "high");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0], "Transaction reverted");
    }

    #[test]
    fn test_assess_risk_unlimited_approval() {
        let simulation = infra::tenderly::SimulationResult {
            success: true,
            gas_used: Some(50000),
            output: "0x".to_string(),
            logs: vec![SimulationLog {
                address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59".to_string(),
                topics: vec![
                    APPROVAL_TOPIC.to_string(),
                    "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23".to_string(),
                    "0x000000000000000000000000145863eb42cf62847a6ca784e6416c1682b1b2ae".to_string(),
                ],
                data: "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
            }],
            internal_calls: vec![],
            error_message: None,
        };

        let (level, warnings) = assess_risk(&simulation);
        assert_eq!(level, "medium");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Unlimited token approval"));
    }

    #[test]
    fn test_assess_risk_limited_approval() {
        let simulation = infra::tenderly::SimulationResult {
            success: true,
            gas_used: Some(50000),
            output: "0x".to_string(),
            logs: vec![SimulationLog {
                address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59".to_string(),
                topics: vec![
                    APPROVAL_TOPIC.to_string(),
                    "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23".to_string(),
                    "0x000000000000000000000000145863eb42cf62847a6ca784e6416c1682b1b2ae".to_string(),
                ],
                data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            }],
            internal_calls: vec![],
            error_message: None,
        };

        let (level, warnings) = assess_risk(&simulation);
        assert_eq!(level, "low");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_assess_risk_internal_call_failed() {
        let simulation = infra::tenderly::SimulationResult {
            success: true,
            gas_used: Some(50000),
            output: "0x".to_string(),
            logs: vec![],
            internal_calls: vec![InternalCall {
                call_type: "CALL".to_string(),
                from: "0x1111111111111111111111111111111111111111".to_string(),
                to: "0x2222222222222222222222222222222222222222".to_string(),
                value: "0x0".to_string(),
                gas_used: Some(1000),
                input: "0x".to_string(),
                output: "0x".to_string(),
                error: Some("out of gas".to_string()),
            }],
            error_message: None,
        };

        let (level, warnings) = assess_risk(&simulation);
        assert_eq!(level, "low"); // Failed internal call doesn't escalate to medium
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Internal call"));
    }

    // ============ format_internal_calls tests ============

    #[test]
    fn test_format_internal_calls() {
        let calls = vec![
            InternalCall {
                call_type: "CALL".to_string(),
                from: "0x1111111111111111111111111111111111111111".to_string(),
                to: "0x2222222222222222222222222222222222222222".to_string(),
                value: "0x0".to_string(),
                gas_used: Some(21000),
                input: "0xabcd".to_string(),
                output: "0x1234".to_string(),
                error: None,
            },
            InternalCall {
                call_type: "STATICCALL".to_string(),
                from: "0x2222222222222222222222222222222222222222".to_string(),
                to: "0x3333333333333333333333333333333333333333".to_string(),
                value: "0x0".to_string(),
                gas_used: Some(5000),
                input: "0x".to_string(),
                output: "0x".to_string(),
                error: None,
            },
        ];

        let formatted = format_internal_calls(&calls);
        assert_eq!(formatted.len(), 2);
        assert_eq!(formatted[0]["type"], "CALL");
        assert_eq!(formatted[1]["type"], "STATICCALL");
    }

    #[test]
    fn test_format_internal_calls_empty() {
        let calls: Vec<InternalCall> = vec![];
        let formatted = format_internal_calls(&calls);
        assert!(formatted.is_empty());
    }

    // ============ parse_u256_from_hex_slice tests ============

    #[test]
    fn test_parse_u256_from_hex_slice_valid() {
        let data = "0000000000000000000000000000000000000000000000000de0b6b3a7640000"; // 1e18
        let value = parse_u256_from_hex_slice(data, 0);
        assert_eq!(value, U256::from(1_000_000_000_000_000_000u64));
    }

    #[test]
    fn test_parse_u256_from_hex_slice_offset() {
        let data = format!(
            "{}{}",
            "0000000000000000000000000000000000000000000000000000000000000001", // first
            "0000000000000000000000000000000000000000000000000000000000000002"  // second
        );
        let first = parse_u256_from_hex_slice(&data, 0);
        let second = parse_u256_from_hex_slice(&data, 64);
        assert_eq!(first, U256::from(1u64));
        assert_eq!(second, U256::from(2u64));
    }

    #[test]
    fn test_parse_u256_from_hex_slice_short_data() {
        let data = "1234"; // too short
        let value = parse_u256_from_hex_slice(data, 0);
        assert_eq!(value, U256::ZERO);
    }

    #[test]
    fn test_parse_u256_from_hex_slice_offset_overflow() {
        let data = "0000000000000000000000000000000000000000000000000000000000000001";
        let value = parse_u256_from_hex_slice(data, 100); // offset beyond data length
        assert_eq!(value, U256::ZERO);
    }

    // ============ Swap V3 tests ============

    #[test]
    fn test_decode_swap_v3_event() {
        // UniswapV3 Swap event
        // Note: topic_to_address preserves the case of the last 40 chars from topics
        let logs = vec![SimulationLog {
            address: "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8".to_string(), // USDC-ETH pool
            topics: vec![
                SWAP_V3_TOPIC.to_string(),
                // Using lowercase addresses in topics to match expected output
                "0x000000000000000000000000e592427a0aece92de3edee1f18e0157c05861564".to_string(), // sender (router)
                "0x0000000000000000000000005c7f8a570d578ed84e63fdfa7b1ee72deae1ae23".to_string(), // recipient
            ],
            data: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        }];

        let changes = decode_state_changes(&logs);
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change["type"], "swap");
        assert_eq!(change["description"], "DEX Swap (V3)");
        // topic_to_address extracts last 40 chars and preserves original case
        assert_eq!(
            change["sender"],
            "0xe592427a0aece92de3edee1f18e0157c05861564"
        );
        assert_eq!(
            change["recipient"],
            "0x5c7f8a570d578ed84e63fdfa7b1ee72deae1ae23"
        );
        // Pool address comes from log.address which is not modified
        assert_eq!(
            change["pool"],
            "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8"
        );
    }

    // ============ Multi-event combination tests ============

    #[test]
    fn test_decode_multiple_events_transfer_and_swap() {
        // Real scenario: user swaps tokens, which triggers Transfer + Swap events
        let logs = vec![
            // First: Transfer from user to pair
            SimulationLog {
                address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59".to_string(), // USDC
                topics: vec![
                    TRANSFER_TOPIC.to_string(),
                    "0x0000000000000000000000005C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(),
                    "0x000000000000000000000000bF62c67eA509E86F07c8c69d0286C0636C50270b".to_string(),
                ],
                data: "0x00000000000000000000000000000000000000000000000000000000000f4240".to_string(), // 1000000
            },
            // Second: Swap event on the pair
            SimulationLog {
                address: "0xbF62c67eA509E86F07c8c69d0286C0636C50270b".to_string(), // CRO-USDC pair
                topics: vec![
                    SWAP_TOPIC.to_string(),
                    "0x000000000000000000000000145863eb42cf62847a6ca784e6416c1682b1b2ae".to_string(),
                    "0x0000000000000000000000005C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(),
                ],
                data: format!(
                    "0x{}{}{}{}",
                    "00000000000000000000000000000000000000000000000000000000000f4240", // amount0In
                    "0000000000000000000000000000000000000000000000000000000000000000", // amount1In
                    "0000000000000000000000000000000000000000000000000000000000000000", // amount0Out
                    "0000000000000000000000000000000000000000000000000de0b6b3a7640000"  // amount1Out
                ),
            },
            // Third: Transfer from pair to user (output token)
            SimulationLog {
                address: "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(), // WCRO
                topics: vec![
                    TRANSFER_TOPIC.to_string(),
                    "0x000000000000000000000000bF62c67eA509E86F07c8c69d0286C0636C50270b".to_string(),
                    "0x0000000000000000000000005C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(),
                ],
                data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(), // 1e18
            },
        ];

        let changes = decode_state_changes(&logs);
        assert_eq!(changes.len(), 3);

        // Verify order and types
        assert_eq!(changes[0]["type"], "transfer");
        assert_eq!(changes[0]["amount"], "1000000");

        assert_eq!(changes[1]["type"], "swap");
        assert_eq!(changes[1]["description"], "DEX Swap (V2)");

        assert_eq!(changes[2]["type"], "transfer");
        assert_eq!(changes[2]["amount"], "1000000000000000000");
    }

    #[test]
    fn test_decode_approval_then_transfer() {
        // Real scenario: approve + transferFrom pattern
        let logs = vec![
            SimulationLog {
                address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59".to_string(),
                topics: vec![
                    APPROVAL_TOPIC.to_string(),
                    "0x0000000000000000000000005C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(),
                    "0x000000000000000000000000145863eb42cf62847a6ca784e6416c1682b1b2ae".to_string(),
                ],
                data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            },
            SimulationLog {
                address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59".to_string(),
                topics: vec![
                    TRANSFER_TOPIC.to_string(),
                    "0x0000000000000000000000005C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(),
                    "0x0000000000000000000000001234567890123456789012345678901234567890".to_string(),
                ],
                data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            },
        ];

        let changes = decode_state_changes(&logs);
        assert_eq!(changes.len(), 2);

        assert_eq!(changes[0]["type"], "approval");
        assert_eq!(changes[0]["unlimited"], false);

        assert_eq!(changes[1]["type"], "transfer");
    }

    #[test]
    fn test_decode_deposit_then_swap() {
        // Real scenario: wrap native token then swap
        let logs = vec![
            // Deposit (wrap CRO to WCRO)
            SimulationLog {
                address: "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(),
                topics: vec![
                    DEPOSIT_TOPIC.to_string(),
                    "0x0000000000000000000000005C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(),
                ],
                data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            },
            // Then swap WCRO for USDC
            SimulationLog {
                address: "0xbF62c67eA509E86F07c8c69d0286C0636C50270b".to_string(),
                topics: vec![
                    SWAP_TOPIC.to_string(),
                    "0x000000000000000000000000145863eb42cf62847a6ca784e6416c1682b1b2ae".to_string(),
                    "0x0000000000000000000000005C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(),
                ],
                data: format!(
                    "0x{}{}{}{}",
                    "0000000000000000000000000000000000000000000000000de0b6b3a7640000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "00000000000000000000000000000000000000000000000000000000000f4240"
                ),
            },
        ];

        let changes = decode_state_changes(&logs);
        assert_eq!(changes.len(), 2);

        assert_eq!(changes[0]["type"], "deposit");
        assert_eq!(changes[0]["amount"], "1000000000000000000");

        assert_eq!(changes[1]["type"], "swap");
    }

    #[test]
    fn test_assess_risk_multiple_warnings() {
        // Scenario: unlimited approval + failed internal call
        let simulation = infra::tenderly::SimulationResult {
            success: true,
            gas_used: Some(100000),
            output: "0x".to_string(),
            logs: vec![SimulationLog {
                address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59".to_string(),
                topics: vec![
                    APPROVAL_TOPIC.to_string(),
                    "0x0000000000000000000000005C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23".to_string(),
                    "0x000000000000000000000000145863eb42cf62847a6ca784e6416c1682b1b2ae".to_string(),
                ],
                data: "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
            }],
            internal_calls: vec![InternalCall {
                call_type: "CALL".to_string(),
                from: "0x1111111111111111111111111111111111111111".to_string(),
                to: "0x2222222222222222222222222222222222222222".to_string(),
                value: "0x0".to_string(),
                gas_used: Some(1000),
                input: "0x".to_string(),
                output: "0x".to_string(),
                error: Some("out of gas".to_string()),
            }],
            error_message: None,
        };

        let (level, warnings) = assess_risk(&simulation);
        assert_eq!(level, "medium"); // Unlimited approval triggers medium
        assert_eq!(warnings.len(), 2); // Both warnings present
        assert!(warnings.iter().any(|w| w.contains("Unlimited")));
        assert!(warnings.iter().any(|w| w.contains("Internal call")));
    }
}
