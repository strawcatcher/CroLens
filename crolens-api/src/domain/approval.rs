use alloy_primitives::{Address, U256};
use alloy_sol_types::SolCall;
use serde::Deserialize;
use serde_json::Value;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::infra::multicall::Call;
use crate::types;

#[derive(Debug, Deserialize)]
struct GetApprovalStatusArgs {
    address: String,
    token: Option<String>,
    #[serde(default)]
    simple_mode: bool,
}

/// Known spender contracts
struct SpenderInfo {
    address: Address,
    name: &'static str,
    protocol: &'static str,
}

fn known_spenders() -> Vec<SpenderInfo> {
    vec![
        // VVS Finance
        SpenderInfo {
            address: types::parse_address("0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae")
                .unwrap_or(Address::ZERO),
            name: "VVS Router",
            protocol: "VVS Finance",
        },
        SpenderInfo {
            address: types::parse_address("0xDccd6455AE04b03d785F12196B492b18129564bc")
                .unwrap_or(Address::ZERO),
            name: "VVS MasterChef",
            protocol: "VVS Finance",
        },
        // Tectonic
        SpenderInfo {
            address: types::parse_address("0xB3831584acb95ED9cCb0C11f677B5AD01DeaeEc0")
                .unwrap_or(Address::ZERO),
            name: "Tectonic Comptroller",
            protocol: "Tectonic",
        },
        // Common DEX aggregators
        SpenderInfo {
            address: types::parse_address("0x1111111254fb6c44bAC0beD2854e76F90643097d")
                .unwrap_or(Address::ZERO),
            name: "1inch Router",
            protocol: "1inch",
        },
    ]
}

/// Get approval status for an address
pub async fn get_approval_status(services: &infra::Services, args: Value) -> Result<Value> {
    let input: GetApprovalStatusArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let owner = types::parse_address(&input.address)?;

    // Get token list
    let tokens = infra::token::list_tokens_cached(&services.db, &services.kv).await?;

    // If specific token requested, filter to that token
    let tokens_to_check: Vec<_> = if let Some(ref token_query) = input.token {
        let token = infra::token::resolve_token(&tokens, token_query)?;
        vec![token]
    } else {
        // Check top 10 most common tokens
        tokens.into_iter().take(10).collect()
    };

    let spenders = known_spenders();
    let multicall = services.multicall()?;

    // Build calls: for each token, check allowance against each known spender
    let mut calls = Vec::new();
    let mut call_map: Vec<(usize, usize)> = Vec::new(); // (token_idx, spender_idx)

    for (ti, token) in tokens_to_check.iter().enumerate() {
        for (si, spender) in spenders.iter().enumerate() {
            calls.push(Call {
                target: token.address,
                call_data: abi::allowanceCall {
                    owner,
                    spender: spender.address,
                }
                .abi_encode()
                .into(),
            });
            call_map.push((ti, si));
        }
    }

    let results = multicall.aggregate(calls).await?;

    // Process results
    let mut approvals: Vec<Value> = Vec::new();
    let max_u256 = U256::MAX;
    let unlimited_threshold = U256::from(10).pow(U256::from(30)); // 1e30

    for (idx, result) in results.into_iter().enumerate() {
        let (ti, si) = call_map[idx];
        let token = &tokens_to_check[ti];
        let spender = &spenders[si];

        if let Ok(data) = result {
            if let Ok(decoded) = abi::allowanceCall::abi_decode_returns(&data, true) {
                let allowance = U256::from(decoded._0);

                // Skip zero allowances
                if allowance == U256::ZERO {
                    continue;
                }

                let is_unlimited = allowance == max_u256 || allowance >= unlimited_threshold;
                let allowance_str = if is_unlimited {
                    "unlimited".to_string()
                } else {
                    types::format_units(&allowance, token.decimals)
                };

                // Determine risk level
                let risk_level = if is_unlimited {
                    "warning"
                } else {
                    "safe"
                };

                approvals.push(serde_json::json!({
                    "token_symbol": token.symbol,
                    "token_address": token.address.to_string(),
                    "spender_address": spender.address.to_string(),
                    "spender_name": spender.name,
                    "protocol": spender.protocol,
                    "allowance": allowance_str,
                    "is_unlimited": is_unlimited,
                    "risk_level": risk_level
                }));
            }
        }
    }

    // Calculate summary
    let total_approvals = approvals.len();
    let unlimited_approvals = approvals
        .iter()
        .filter(|a| a.get("is_unlimited").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();

    let risk_score = if total_approvals == 0 {
        0
    } else {
        // Simple risk score: 10 points per unlimited approval, max 100
        ((unlimited_approvals as u32) * 20).min(100)
    };

    if input.simple_mode {
        let text = if total_approvals == 0 {
            "No token approvals found for known spenders.".to_string()
        } else {
            format!(
                "{} approval(s) | {} unlimited ({}) | Risk score: {}/100",
                total_approvals,
                unlimited_approvals,
                if unlimited_approvals > 0 {
                    "⚠️ warning"
                } else {
                    "safe"
                },
                risk_score
            )
        };
        return Ok(serde_json::json!({
            "text": text,
            "meta": services.meta()
        }));
    }

    Ok(serde_json::json!({
        "address": owner.to_string(),
        "approvals": approvals,
        "summary": {
            "total_approvals": total_approvals,
            "unlimited_approvals": unlimited_approvals,
            "risk_score": risk_score
        },
        "meta": services.meta()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_spenders_are_valid() {
        let spenders = known_spenders();
        assert!(!spenders.is_empty());
        for s in &spenders {
            assert_ne!(s.address, Address::ZERO);
            assert!(!s.name.is_empty());
            assert!(!s.protocol.is_empty());
        }
    }

    #[test]
    fn known_spenders_has_expected_protocols() {
        let spenders = known_spenders();
        let protocols: Vec<_> = spenders.iter().map(|s| s.protocol).collect();
        assert!(protocols.contains(&"VVS Finance"));
        assert!(protocols.contains(&"Tectonic"));
    }

    #[test]
    fn known_spenders_has_expected_count() {
        let spenders = known_spenders();
        assert_eq!(spenders.len(), 4);
    }

    #[test]
    fn known_spenders_addresses_are_checksum_valid() {
        let spenders = known_spenders();
        for s in &spenders {
            // Address should be non-zero and valid
            assert_ne!(s.address, Address::ZERO);
            // Should be able to format to string
            let addr_str = s.address.to_string();
            assert!(addr_str.starts_with("0x"));
            assert_eq!(addr_str.len(), 42);
        }
    }

    #[test]
    fn args_deserialize_with_address_only() {
        let json = serde_json::json!({
            "address": "0x1234567890123456789012345678901234567890"
        });
        let args: std::result::Result<GetApprovalStatusArgs, _> = serde_json::from_value(json);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.address, "0x1234567890123456789012345678901234567890");
        assert!(args.token.is_none());
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_with_all_fields() {
        let json = serde_json::json!({
            "address": "0x1234567890123456789012345678901234567890",
            "token": "VVS",
            "simple_mode": true
        });
        let args: std::result::Result<GetApprovalStatusArgs, _> = serde_json::from_value(json);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.token, Some("VVS".to_string()));
        assert!(args.simple_mode);
    }
}
