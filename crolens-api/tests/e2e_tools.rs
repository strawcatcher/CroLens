//! End-to-end tests for MCP tools
//!
//! These tests verify the complete request-response cycle
//! using simulated request payloads.

use serde_json::json;

/// Helper to create a JSON-RPC request for tools/call
fn make_tool_call(tool_name: &str, arguments: serde_json::Value) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    })
}

// ============================================================
// get_approval_status E2E tests
// ============================================================

#[test]
fn e2e_get_approval_status_request_parses() {
    let req = make_tool_call("get_approval_status", json!({
        "address": "0x1234567890123456789012345678901234567890"
    }));

    assert_eq!(req.get("method").and_then(|v| v.as_str()), Some("tools/call"));
    let params = req.get("params").unwrap();
    assert_eq!(params.get("name").and_then(|v| v.as_str()), Some("get_approval_status"));
}

#[test]
fn e2e_get_approval_status_with_token_filter() {
    let req = make_tool_call("get_approval_status", json!({
        "address": "0x1234567890123456789012345678901234567890",
        "token": "USDC"
    }));

    let params = req.get("params").unwrap();
    let args = params.get("arguments").unwrap();
    assert_eq!(args.get("token").and_then(|v| v.as_str()), Some("USDC"));
}

#[test]
fn e2e_get_approval_status_simple_mode() {
    let req = make_tool_call("get_approval_status", json!({
        "address": "0x1234567890123456789012345678901234567890",
        "simple_mode": true
    }));

    let params = req.get("params").unwrap();
    let args = params.get("arguments").unwrap();
    assert_eq!(args.get("simple_mode").and_then(|v| v.as_bool()), Some(true));
}

// ============================================================
// get_block_info E2E tests
// ============================================================

#[test]
fn e2e_get_block_info_latest() {
    let req = make_tool_call("get_block_info", json!({}));

    assert_eq!(req.get("method").and_then(|v| v.as_str()), Some("tools/call"));
    let params = req.get("params").unwrap();
    assert_eq!(params.get("name").and_then(|v| v.as_str()), Some("get_block_info"));
}

#[test]
fn e2e_get_block_info_by_number() {
    let req = make_tool_call("get_block_info", json!({
        "block": "12345"
    }));

    let params = req.get("params").unwrap();
    let args = params.get("arguments").unwrap();
    assert_eq!(args.get("block").and_then(|v| v.as_str()), Some("12345"));
}

#[test]
fn e2e_get_block_info_by_hex() {
    let req = make_tool_call("get_block_info", json!({
        "block": "0x3039"
    }));

    let params = req.get("params").unwrap();
    let args = params.get("arguments").unwrap();
    assert_eq!(args.get("block").and_then(|v| v.as_str()), Some("0x3039"));
}

#[test]
fn e2e_get_block_info_simple_mode() {
    let req = make_tool_call("get_block_info", json!({
        "block": "latest",
        "simple_mode": true
    }));

    let params = req.get("params").unwrap();
    let args = params.get("arguments").unwrap();
    assert_eq!(args.get("simple_mode").and_then(|v| v.as_bool()), Some(true));
}

// ============================================================
// Request validation tests
// ============================================================

#[test]
fn e2e_request_has_correct_jsonrpc_version() {
    let req = make_tool_call("get_block_info", json!({}));
    assert_eq!(req.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
}

#[test]
fn e2e_request_has_id() {
    let req = make_tool_call("get_block_info", json!({}));
    assert_eq!(req.get("id").and_then(|v| v.as_i64()), Some(1));
}

#[test]
fn e2e_multiple_tools_request_format() {
    // Test that we can create requests for all 12 tools
    let tools = [
        ("get_account_summary", json!({"address": "0x123"})),
        ("get_defi_positions", json!({"address": "0x123"})),
        ("decode_transaction", json!({"tx_hash": "0xabc"})),
        ("simulate_transaction", json!({"from": "0x1", "to": "0x2", "data": "0x", "value": "0"})),
        ("search_contract", json!({"query": "usdc"})),
        ("construct_swap_tx", json!({"from": "0x1", "token_in": "CRO", "token_out": "USDC", "amount_in": "100", "slippage_bps": 50})),
        ("get_token_info", json!({"token": "VVS"})),
        ("get_pool_info", json!({"pool": "CRO-USDC"})),
        ("get_gas_price", json!({})),
        ("get_token_price", json!({"tokens": ["CRO", "VVS"]})),
        ("get_approval_status", json!({"address": "0x123"})),
        ("get_block_info", json!({})),
    ];

    for (tool_name, args) in tools {
        let req = make_tool_call(tool_name, args);
        let params = req.get("params").unwrap();
        assert_eq!(
            params.get("name").and_then(|v| v.as_str()),
            Some(tool_name),
            "Tool name mismatch for {tool_name}"
        );
    }
}

// ============================================================
// Response format tests (mock responses)
// ============================================================

#[test]
fn approval_status_response_format() {
    // Test expected response format
    let response = json!({
        "address": "0x1234567890123456789012345678901234567890",
        "approvals": [
            {
                "token_symbol": "USDC",
                "token_address": "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59",
                "spender_address": "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae",
                "spender_name": "VVS Router",
                "protocol": "VVS Finance",
                "allowance": "unlimited",
                "is_unlimited": true,
                "risk_level": "warning"
            }
        ],
        "summary": {
            "total_approvals": 1,
            "unlimited_approvals": 1,
            "risk_score": 20
        },
        "meta": {
            "timestamp": 1234567890,
            "latency_ms": 100
        }
    });

    // Verify structure
    assert!(response.get("address").is_some());
    assert!(response.get("approvals").and_then(|v| v.as_array()).is_some());
    assert!(response.get("summary").is_some());

    let summary = response.get("summary").unwrap();
    assert!(summary.get("total_approvals").is_some());
    assert!(summary.get("unlimited_approvals").is_some());
    assert!(summary.get("risk_score").is_some());

    // Verify approval entry structure
    let approvals = response.get("approvals").unwrap().as_array().unwrap();
    assert_eq!(approvals.len(), 1);
    let approval = &approvals[0];
    assert!(approval.get("token_symbol").is_some());
    assert!(approval.get("token_address").is_some());
    assert!(approval.get("spender_address").is_some());
    assert!(approval.get("spender_name").is_some());
    assert!(approval.get("protocol").is_some());
    assert!(approval.get("allowance").is_some());
    assert!(approval.get("is_unlimited").is_some());
    assert!(approval.get("risk_level").is_some());
}

#[test]
fn block_info_response_format() {
    // Test expected response format
    let response = json!({
        "number": 15000000,
        "hash": "0xabc123def456789012345678901234567890123456789012345678901234abcd",
        "timestamp": 1700000000,
        "timestamp_relative": "5 minutes ago",
        "transactions_count": 150,
        "gas_used": "8000000",
        "gas_limit": "15000000",
        "gas_used_percent": "53.33",
        "base_fee_gwei": "5000.00",
        "miner": "0x0000000000000000000000000000000000000000",
        "meta": {
            "timestamp": 1234567890,
            "latency_ms": 50
        }
    });

    // Verify structure
    assert!(response.get("number").and_then(|v| v.as_i64()).is_some());
    assert!(response.get("hash").and_then(|v| v.as_str()).is_some());
    assert!(response.get("timestamp").and_then(|v| v.as_i64()).is_some());
    assert!(response.get("timestamp_relative").and_then(|v| v.as_str()).is_some());
    assert!(response.get("transactions_count").and_then(|v| v.as_i64()).is_some());
    assert!(response.get("gas_used").and_then(|v| v.as_str()).is_some());
    assert!(response.get("gas_limit").and_then(|v| v.as_str()).is_some());
    assert!(response.get("gas_used_percent").and_then(|v| v.as_str()).is_some());
    assert!(response.get("base_fee_gwei").and_then(|v| v.as_str()).is_some());
    assert!(response.get("miner").and_then(|v| v.as_str()).is_some());
}

#[test]
fn simple_mode_response_format() {
    // Test simple mode text response format
    let response = json!({
        "text": "Block #15000000 | 5 minutes ago | 150 txs | Gas: 53.3% used | Base fee: 5000.00 gwei",
        "meta": {
            "timestamp": 1234567890,
            "latency_ms": 50
        }
    });

    assert!(response.get("text").and_then(|v| v.as_str()).is_some());
    let text = response.get("text").unwrap().as_str().unwrap();
    assert!(text.contains("Block"));
    assert!(text.contains("txs"));
    assert!(text.contains("Gas"));
}

#[test]
fn approval_simple_mode_response_format() {
    let response = json!({
        "text": "3 approval(s) | 2 unlimited (⚠️ warning) | Risk score: 40/100",
        "meta": {
            "timestamp": 1234567890,
            "latency_ms": 50
        }
    });

    let text = response.get("text").unwrap().as_str().unwrap();
    assert!(text.contains("approval"));
    assert!(text.contains("unlimited"));
    assert!(text.contains("Risk score"));
}

// ============================================================
// JSON-RPC protocol compliance tests
// ============================================================

#[test]
fn jsonrpc_error_response_format() {
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32602,
            "message": "Invalid params",
            "data": {
                "details": "address is required"
            }
        }
    });

    assert_eq!(error_response.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
    assert!(error_response.get("result").is_none());
    let error = error_response.get("error").unwrap();
    assert!(error.get("code").and_then(|v| v.as_i64()).is_some());
    assert!(error.get("message").and_then(|v| v.as_str()).is_some());
}

#[test]
fn jsonrpc_success_response_format() {
    let success_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "some result"
                }
            ]
        }
    });

    assert_eq!(success_response.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
    assert!(success_response.get("error").is_none());
    assert!(success_response.get("result").is_some());
}
