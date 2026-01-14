use alloy_sol_types::SolCall;
use serde::Deserialize;
use serde_json::Value;
use worker::d1::D1Type;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct DecodeArgs {
    tx_hash: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn decode_transaction(services: &infra::Services, args: Value) -> Result<Value> {
    let input: DecodeArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let hash = input.tx_hash.trim();
    types::validate_hex_string(hash, 64)?;

    let rpc = services.rpc()?;
    let tx = rpc.eth_get_transaction_by_hash(hash).await?;
    let receipt = rpc.eth_get_transaction_receipt(hash).await?;

    let from = tx.get("from").and_then(|v| v.as_str()).unwrap_or_default();
    let to = tx.get("to").and_then(|v| v.as_str()).unwrap_or_default();
    let input_data = tx.get("input").and_then(|v| v.as_str()).unwrap_or("0x");

    let selector = input_data.get(0..10).unwrap_or("0x");
    let (action, method_name, decoded_params) = decode_selector(selector, input_data)?;

    let status = receipt
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0")
        .to_string();
    let gas_used = receipt
        .get("gasUsed")
        .and_then(|v| v.as_str())
        .and_then(|v| types::parse_u256_hex(v).ok())
        .map(|u| u.to_string())
        .unwrap_or_else(|| "0".to_string());

    if input.simple_mode {
        let summary = format!("{action}: {method_name} | Status: {status} | Gas: {gas_used}");
        return Ok(serde_json::json!({ "text": summary, "meta": services.meta() }));
    }

    Ok(serde_json::json!({
        "hash": hash,
        "from": from,
        "to": to,
        "action": action,
        "protocol": infer_protocol(&services.db, to).await.unwrap_or(None),
        "status": status,
        "gas_used": gas_used,
        "decoded": {
            "method_name": method_name,
            "params": decoded_params,
        },
        "meta": services.meta(),
    }))
}

fn decode_selector(selector: &str, input_data: &str) -> Result<(String, String, Value)> {
    let bytes = types::hex0x_to_bytes(input_data)?;
    if bytes.len() < 4 {
        return Ok(("Unknown".to_string(), "unknown".to_string(), Value::Null));
    }

    match selector {
        "0xa9059cbb" => {
            let params = match abi::transferCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "to": decoded.recipient.to_string(),
                    "amount": decoded.amount.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok(("Transfer".to_string(), "transfer".to_string(), params))
        }
        "0x23b872dd" => {
            let params = match abi::transferFromCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "from": decoded.sender.to_string(),
                    "to": decoded.recipient.to_string(),
                    "amount": decoded.amount.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok(("Transfer".to_string(), "transferFrom".to_string(), params))
        }
        "0x095ea7b3" => {
            let params = match abi::approveCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "spender": decoded.spender.to_string(),
                    "amount": decoded.amount.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok(("Approve".to_string(), "approve".to_string(), params))
        }
        "0x38ed1739" => {
            let params = match abi::swapExactTokensForTokensCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "amount_in": decoded.amountIn.to_string(),
                    "amount_out_min": decoded.amountOutMin.to_string(),
                    "path": decoded.path.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                    "to": decoded.to.to_string(),
                    "deadline": decoded.deadline.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok((
                "Swap".to_string(),
                "swapExactTokensForTokens".to_string(),
                params,
            ))
        }
        "0x7ff36ab5" => {
            let params = match abi::swapExactETHForTokensCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "amount_out_min": decoded.amountOutMin.to_string(),
                    "path": decoded.path.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                    "to": decoded.to.to_string(),
                    "deadline": decoded.deadline.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok((
                "Swap".to_string(),
                "swapExactETHForTokens".to_string(),
                params,
            ))
        }
        "0x18cbafe5" => {
            let params = match abi::swapExactTokensForETHCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "amount_in": decoded.amountIn.to_string(),
                    "amount_out_min": decoded.amountOutMin.to_string(),
                    "path": decoded.path.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                    "to": decoded.to.to_string(),
                    "deadline": decoded.deadline.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok((
                "Swap".to_string(),
                "swapExactTokensForETH".to_string(),
                params,
            ))
        }
        "0x8803dbee" => {
            let params = match abi::swapTokensForExactTokensCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "amount_out": decoded.amountOut.to_string(),
                    "amount_in_max": decoded.amountInMax.to_string(),
                    "path": decoded.path.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                    "to": decoded.to.to_string(),
                    "deadline": decoded.deadline.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok((
                "Swap".to_string(),
                "swapTokensForExactTokens".to_string(),
                params,
            ))
        }
        "0xfb3bdb41" => {
            let params = match abi::swapETHForExactTokensCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "amount_out": decoded.amountOut.to_string(),
                    "path": decoded.path.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                    "to": decoded.to.to_string(),
                    "deadline": decoded.deadline.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok((
                "Swap".to_string(),
                "swapETHForExactTokens".to_string(),
                params,
            ))
        }
        "0x4a25d94a" => {
            let params = match abi::swapTokensForExactETHCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "amount_out": decoded.amountOut.to_string(),
                    "amount_in_max": decoded.amountInMax.to_string(),
                    "path": decoded.path.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                    "to": decoded.to.to_string(),
                    "deadline": decoded.deadline.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok((
                "Swap".to_string(),
                "swapTokensForExactETH".to_string(),
                params,
            ))
        }
        "0xe8e33700" => {
            let params = match abi::addLiquidityCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "token_a": decoded.tokenA.to_string(),
                    "token_b": decoded.tokenB.to_string(),
                    "amount_a_desired": decoded.amountADesired.to_string(),
                    "amount_b_desired": decoded.amountBDesired.to_string(),
                    "amount_a_min": decoded.amountAMin.to_string(),
                    "amount_b_min": decoded.amountBMin.to_string(),
                    "to": decoded.to.to_string(),
                    "deadline": decoded.deadline.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok(("Liquidity".to_string(), "addLiquidity".to_string(), params))
        }
        "0xf305d719" => {
            let params = match abi::addLiquidityETHCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "token": decoded.token.to_string(),
                    "amount_token_desired": decoded.amountTokenDesired.to_string(),
                    "amount_token_min": decoded.amountTokenMin.to_string(),
                    "amount_eth_min": decoded.amountETHMin.to_string(),
                    "to": decoded.to.to_string(),
                    "deadline": decoded.deadline.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok((
                "Liquidity".to_string(),
                "addLiquidityETH".to_string(),
                params,
            ))
        }
        "0xbaa2abde" => {
            let params = match abi::removeLiquidityCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "token_a": decoded.tokenA.to_string(),
                    "token_b": decoded.tokenB.to_string(),
                    "liquidity": decoded.liquidity.to_string(),
                    "amount_a_min": decoded.amountAMin.to_string(),
                    "amount_b_min": decoded.amountBMin.to_string(),
                    "to": decoded.to.to_string(),
                    "deadline": decoded.deadline.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok((
                "Liquidity".to_string(),
                "removeLiquidity".to_string(),
                params,
            ))
        }
        "0x02751cec" => {
            let params = match abi::removeLiquidityETHCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "token": decoded.token.to_string(),
                    "liquidity": decoded.liquidity.to_string(),
                    "amount_token_min": decoded.amountTokenMin.to_string(),
                    "amount_eth_min": decoded.amountETHMin.to_string(),
                    "to": decoded.to.to_string(),
                    "deadline": decoded.deadline.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok((
                "Liquidity".to_string(),
                "removeLiquidityETH".to_string(),
                params,
            ))
        }
        "0xa0712d68" => {
            let params = match abi::mintCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "mint_amount": decoded.mintAmount.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok(("Lending".to_string(), "mint".to_string(), params))
        }
        "0xdb006a75" => {
            let params = match abi::redeemCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "redeem_tokens": decoded.redeemTokens.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok(("Lending".to_string(), "redeem".to_string(), params))
        }
        "0x852a12e3" => {
            let params = match abi::redeemUnderlyingCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "redeem_amount": decoded.redeemAmount.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok((
                "Lending".to_string(),
                "redeemUnderlying".to_string(),
                params,
            ))
        }
        "0xc5ebeaec" => {
            let params = match abi::borrowCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "borrow_amount": decoded.borrowAmount.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok(("Lending".to_string(), "borrow".to_string(), params))
        }
        "0x0e752702" => {
            let params = match abi::repayBorrowCall::abi_decode(&bytes, true) {
                Ok(decoded) => serde_json::json!({
                    "repay_amount": decoded.repayAmount.to_string(),
                }),
                Err(_) => Value::Null,
            };
            Ok(("Lending".to_string(), "repayBorrow".to_string(), params))
        }
        _ => Ok(("Unknown".to_string(), "unknown".to_string(), Value::Null)),
    }
}

async fn infer_protocol(db: &worker::D1Database, address: &str) -> Result<Option<String>> {
    if address.is_empty() {
        return Ok(None);
    }

    let address_arg = D1Type::Text(address);
    let statement = db
        .prepare("SELECT protocol_id FROM contracts WHERE address = ?1 LIMIT 1")
        .bind_refs([&address_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let result = infra::db::run("infer_protocol", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let Some(row) = rows.first() else {
        return Ok(None);
    };

    Ok(row
        .get("protocol_id")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;

    #[test]
    fn decodes_erc20_transfer_params() {
        let recipient = types::parse_address("0x1111111111111111111111111111111111111111").unwrap();
        let amount = U256::from(42u64);
        let calldata = abi::transferCall { recipient, amount }.abi_encode();
        let input_hex = types::bytes_to_hex0x(&calldata);

        let (action, method, params) = decode_selector("0xa9059cbb", &input_hex).unwrap();
        assert_eq!(action, "Transfer");
        assert_eq!(method, "transfer");

        let recipient_str = recipient.to_string();
        assert_eq!(
            params.get("to").and_then(|v| v.as_str()),
            Some(recipient_str.as_str())
        );
        assert_eq!(params.get("amount").and_then(|v| v.as_str()), Some("42"));
    }

    #[test]
    fn decodes_swap_exact_tokens_for_tokens_params() {
        let to = types::parse_address("0x2222222222222222222222222222222222222222").unwrap();
        let token_a = types::parse_address("0x3333333333333333333333333333333333333333").unwrap();
        let token_b = types::parse_address("0x4444444444444444444444444444444444444444").unwrap();
        let calldata = abi::swapExactTokensForTokensCall {
            amountIn: U256::from(1000u64),
            amountOutMin: U256::from(900u64),
            path: vec![token_a, token_b],
            to,
            deadline: U256::from(123u64),
        }
        .abi_encode();
        let input_hex = types::bytes_to_hex0x(&calldata);

        let (action, method, params) = decode_selector("0x38ed1739", &input_hex).unwrap();
        assert_eq!(action, "Swap");
        assert_eq!(method, "swapExactTokensForTokens");
        assert_eq!(
            params.get("amount_in").and_then(|v| v.as_str()),
            Some("1000")
        );
        assert_eq!(
            params
                .get("path")
                .and_then(|v| v.as_array())
                .map(|v| v.len()),
            Some(2)
        );
    }
}
