use serde_json::Value;

use crate::mcp::protocol::ToolDefinition;

pub fn list() -> Value {
    Value::Object(
        [(
            "tools".to_string(),
            Value::Array(
                tool_definitions()
                    .into_iter()
                    .map(|t| serde_json::to_value(t).unwrap_or(Value::Null))
                    .collect(),
            ),
        )]
        .into_iter()
        .collect(),
    )
}

fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_account_summary".to_string(),
            description: "Complete account overview: wallet balances + DeFi summary.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "address": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["address"]
            }),
        },
        ToolDefinition {
            name: "get_defi_positions".to_string(),
            description: "Detailed DeFi positions (VVS LP, Tectonic supply/borrow).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "address": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["address"]
            }),
        },
        ToolDefinition {
            name: "decode_transaction".to_string(),
            description: "Translate transaction hash to human-readable action.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tx_hash": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["tx_hash"]
            }),
        },
        ToolDefinition {
            name: "simulate_transaction".to_string(),
            description: "Simulate transaction execution and return state changes + risk hints."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "from": { "type": "string" },
                    "to": { "type": "string" },
                    "data": { "type": "string" },
                    "value": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["from", "to", "data", "value"]
            }),
        },
        ToolDefinition {
            name: "search_contract".to_string(),
            description: "Search contracts by name, symbol, or address.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "construct_swap_tx".to_string(),
            description: "Build swap calldata with approval handling.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "from": { "type": "string" },
                    "token_in": { "type": "string" },
                    "token_out": { "type": "string" },
                    "amount_in": { "type": "string" },
                    "slippage_bps": { "type": "integer", "minimum": 0, "maximum": 5000 }
                },
                "required": ["from", "token_in", "token_out", "amount_in", "slippage_bps"]
            }),
        },
        // New tools
        ToolDefinition {
            name: "get_token_info".to_string(),
            description: "Get detailed token information including price, supply, and liquidity."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "token": { "type": "string", "description": "Token symbol (e.g. 'VVS') or address" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["token"]
            }),
        },
        ToolDefinition {
            name: "get_pool_info".to_string(),
            description: "Get LP pool details including TVL, reserves, and APY.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pool": { "type": "string", "description": "Pool pair (e.g. 'CRO-USDC') or LP address" },
                    "dex": { "type": "string", "description": "DEX name (default: 'vvs')" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["pool"]
            }),
        },
        ToolDefinition {
            name: "get_gas_price".to_string(),
            description: "Get current gas price and estimated costs for common operations."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "simple_mode": { "type": "boolean" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_token_price".to_string(),
            description: "Get USD prices for multiple tokens (max 20).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tokens": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Array of token symbols or addresses",
                        "maxItems": 20
                    },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["tokens"]
            }),
        },
        ToolDefinition {
            name: "get_approval_status".to_string(),
            description: "Check token approval status for known spenders (DEX routers, lending protocols)."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "address": { "type": "string", "description": "Wallet address to check" },
                    "token": { "type": "string", "description": "Optional: specific token to check" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["address"]
            }),
        },
        ToolDefinition {
            name: "get_block_info".to_string(),
            description: "Get block information (number, timestamp, gas usage, transactions count)."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "block": { "type": "string", "description": "Block number, hash, or 'latest'" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": []
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_has_expected_shape() {
        let value = list();
        let tools = value
            .get("tools")
            .and_then(|v| v.as_array())
            .expect("tools must be an array");
        assert_eq!(tools.len(), 12); // 6 original + 6 new
        for tool in tools {
            assert!(tool.get("name").and_then(|v| v.as_str()).is_some());
            assert!(tool.get("description").and_then(|v| v.as_str()).is_some());
            assert!(tool.get("inputSchema").is_some());
        }
    }

    #[test]
    fn tools_list_includes_core_tools() {
        let value = list();
        let tools = value
            .get("tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let names = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|v| v.as_str()))
            .collect::<Vec<_>>();

        for required in [
            "get_account_summary",
            "get_defi_positions",
            "decode_transaction",
            "simulate_transaction",
            "search_contract",
            "construct_swap_tx",
            "get_token_info",
            "get_pool_info",
            "get_gas_price",
            "get_token_price",
            "get_approval_status",
            "get_block_info",
        ] {
            assert!(names.contains(&required));
        }
    }
}
