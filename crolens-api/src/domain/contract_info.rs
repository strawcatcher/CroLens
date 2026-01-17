use serde::Deserialize;
use serde_json::Value;
use worker::d1::D1Type;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct ContractInfoArgs {
    address: String,
    #[serde(default)]
    simple_mode: bool,
}

fn code_size_from_hex(code_hex: &str) -> Option<usize> {
    if !code_hex.starts_with("0x") {
        return None;
    }
    Some(code_hex.len().saturating_sub(2) / 2)
}

pub async fn get_contract_info(services: &infra::Services, args: Value) -> Result<Value> {
    let input: ContractInfoArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let addr = types::parse_address(&input.address)?;
    let addr_lower = addr.to_string().to_lowercase();
    let addr_arg = D1Type::Text(&addr_lower);

    let statement = services
        .db
        .prepare(
            "SELECT address, name, type, protocol_id, verified, description \
             FROM contracts WHERE lower(address) = ?1 LIMIT 1",
        )
        .bind_refs([&addr_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let result = infra::db::run("get_contract_info", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let (name, contract_type, protocol_id, verified, description) = rows
        .first()
        .map(|row| {
            (
                row.get("name").and_then(|v| v.as_str()).map(|v| v.to_string()),
                row.get("type").and_then(|v| v.as_str()).map(|v| v.to_string()),
                row.get("protocol_id")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string()),
                row.get("verified").and_then(|v| v.as_i64()).unwrap_or(0) == 1,
                row.get("description")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string()),
            )
        })
        .unwrap_or((None, None, None, false, None));

    // Optional best-effort code size via RPC.
    let mut code_size: Option<usize> = None;
    if let Ok(rpc) = services.rpc() {
        if let Ok(code) = rpc
            .call("eth_getCode", serde_json::json!([addr.to_string(), "latest"]))
            .await
        {
            if let Some(code_hex) = code.as_str() {
                code_size = code_size_from_hex(code_hex);
            }
        }
    }

    if input.simple_mode {
        let text = match name.as_ref() {
            Some(n) => format!("Contract: {n} ({})", addr),
            None => format!("Contract: {addr}"),
        };
        return Ok(serde_json::json!({ "text": text, "meta": services.meta() }));
    }

    Ok(serde_json::json!({
        "address": addr.to_string(),
        "name": name,
        "type": contract_type,
        "protocol": protocol_id,
        "verified": verified,
        "description": description,
        "code_size": code_size,
        "meta": services.meta(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_size_from_hex_variants() {
        assert_eq!(code_size_from_hex("0x"), Some(0));
        assert_eq!(code_size_from_hex("0x00"), Some(1));
        assert_eq!(code_size_from_hex("0x6000"), Some(2));
        assert_eq!(code_size_from_hex("6000"), None);
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({ "address": "0x1234567890123456789012345678901234567890" });
        let args: ContractInfoArgs = serde_json::from_value(json).expect("args should parse");
        assert_eq!(args.address, "0x1234567890123456789012345678901234567890");
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({
            "address": "0x1234567890123456789012345678901234567890",
            "simple_mode": true
        });
        let args: ContractInfoArgs = serde_json::from_value(json).expect("args should parse");
        assert!(args.simple_mode);
    }
}
