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
                    "gas": { "type": "integer" },
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
        ToolDefinition {
            name: "estimate_gas".to_string(),
            description: "Estimate gas for a transaction and show the cost in CRO/USD.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "from": { "type": "string" },
                    "to": { "type": "string" },
                    "data": { "type": "string" },
                    "value": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["from", "to"]
            }),
        },
        ToolDefinition {
            name: "decode_calldata".to_string(),
            description: "Decode calldata into method signature and parameters.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "data": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["data"]
            }),
        },
        ToolDefinition {
            name: "get_vvs_farms".to_string(),
            description: "List VVS farms with estimated TVL and APY.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "simple_mode": { "type": "boolean" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_vvs_rewards".to_string(),
            description: "Get pending VVS rewards for a wallet.".to_string(),
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
            name: "get_tectonic_markets".to_string(),
            description: "List Tectonic lending markets overview.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "simple_mode": { "type": "boolean" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_tectonic_rates".to_string(),
            description: "Compare Tectonic supply/borrow rates (optionally filter by asset).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "asset": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "construct_revoke_approval".to_string(),
            description: "Construct calldata to revoke a token approval for a spender.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "token": { "type": "string" },
                    "spender": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["token", "spender"]
            }),
        },
        ToolDefinition {
            name: "get_lending_rates".to_string(),
            description: "Get lending rates across supported protocols (currently: Tectonic).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "asset": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_cro_overview".to_string(),
            description: "Get CRO overview: price, gas, and network status.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "simple_mode": { "type": "boolean" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_liquidation_risk".to_string(),
            description: "Assess liquidation risk for a wallet's lending positions.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "address": { "type": "string" },
                    "protocol": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["address"]
            }),
        },
        ToolDefinition {
            name: "get_health_alerts".to_string(),
            description: "Aggregate health alerts for balances, approvals, and DeFi positions.".to_string(),
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
            name: "get_best_swap_route".to_string(),
            description: "Find the best swap route for a given trade.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "token_in": { "type": "string" },
                    "token_out": { "type": "string" },
                    "amount_in": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["token_in", "token_out", "amount_in"]
            }),
        },
        ToolDefinition {
            name: "get_protocol_stats".to_string(),
            description: "Get protocol stats such as TVL for VVS and Tectonic.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "protocol": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "resolve_cronos_id".to_string(),
            description: "Resolve .cro domains to addresses (and reverse lookup).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "get_token_approvals".to_string(),
            description: "List token approvals across known spenders.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "address": { "type": "string" },
                    "include_zero": { "type": "boolean" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["address"]
            }),
        },
        ToolDefinition {
            name: "get_contract_info".to_string(),
            description: "Get contract information including type and code size.".to_string(),
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
            name: "get_whale_activity".to_string(),
            description: "Monitor large transfer activity for major tokens.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "token": { "type": "string" },
                    "min_value_usd": { "type": "number" },
                    "blocks": { "type": "integer" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_portfolio_analysis".to_string(),
            description: "Analyze a wallet portfolio and provide diversification insights.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "address": { "type": "string" },
                    "simple_mode": { "type": "boolean" }
                },
                "required": ["address"]
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
        assert_eq!(tools.len(), 30);
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
            "estimate_gas",
            "decode_calldata",
            "get_vvs_farms",
            "get_vvs_rewards",
            "get_tectonic_markets",
            "get_tectonic_rates",
            "construct_revoke_approval",
            "get_lending_rates",
            "get_cro_overview",
            "get_liquidation_risk",
            "get_health_alerts",
            "get_best_swap_route",
            "get_protocol_stats",
            "resolve_cronos_id",
            "get_token_approvals",
            "get_contract_info",
            "get_whale_activity",
            "get_portfolio_analysis",
        ] {
            assert!(names.contains(&required));
        }
    }
}
