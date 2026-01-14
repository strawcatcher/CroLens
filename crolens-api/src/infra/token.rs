use alloy_primitives::Address;
use serde_json::Value;
use worker::d1::D1Type;
use worker::D1Database;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Clone)]
pub struct Token {
    pub address: Address,
    pub symbol: String,
    pub decimals: u8,
    pub is_stablecoin: bool,
}

pub async fn list_tokens(db: &D1Database) -> Result<Vec<Token>> {
    let statement = db.prepare("SELECT address, symbol, decimals, is_stablecoin FROM tokens");
    let result = infra::db::run("list_tokens", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let mut tokens = Vec::with_capacity(rows.len());
    for row in rows {
        let address = row
            .get("address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CroLensError::DbError("tokens.address missing".to_string()))?;
        let symbol = row
            .get("symbol")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CroLensError::DbError("tokens.symbol missing".to_string()))?;
        let decimals = row
            .get("decimals")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| CroLensError::DbError("tokens.decimals missing".to_string()))?;

        let is_stablecoin = match row.get("is_stablecoin") {
            Some(Value::Bool(v)) => *v,
            Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
            _ => false,
        };

        tokens.push(Token {
            address: types::parse_address(address)?,
            symbol: symbol.to_string(),
            decimals: decimals as u8,
            is_stablecoin,
        });
    }

    Ok(tokens)
}

pub async fn get_token_by_address(db: &D1Database, address: Address) -> Result<Option<Token>> {
    let address_str = address.to_string();
    let address_arg = D1Type::Text(&address_str);

    let statement = db
        .prepare("SELECT address, symbol, decimals, is_stablecoin FROM tokens WHERE address = ?1 LIMIT 1")
        .bind_refs([&address_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let result = infra::db::run("get_token_by_address", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let Some(row) = rows.first() else {
        return Ok(None);
    };

    let address = row
        .get("address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CroLensError::DbError("tokens.address missing".to_string()))?;
    let symbol = row
        .get("symbol")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CroLensError::DbError("tokens.symbol missing".to_string()))?;
    let decimals = row
        .get("decimals")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| CroLensError::DbError("tokens.decimals missing".to_string()))?;

    let is_stablecoin = match row.get("is_stablecoin") {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    };

    Ok(Some(Token {
        address: types::parse_address(address)?,
        symbol: symbol.to_string(),
        decimals: decimals as u8,
        is_stablecoin,
    }))
}

pub fn resolve_token(tokens: &[Token], query: &str) -> Result<Token> {
    let trimmed = query.trim();
    if trimmed.starts_with("0x") {
        let addr = types::parse_address(trimmed)?;
        return tokens
            .iter()
            .find(|t| t.address == addr)
            .cloned()
            .ok_or_else(|| CroLensError::TokenNotFound(trimmed.to_string()));
    }

    let wanted = trimmed.to_lowercase();
    tokens
        .iter()
        .find(|t| t.symbol.to_lowercase() == wanted)
        .cloned()
        .ok_or_else(|| CroLensError::TokenNotFound(trimmed.to_string()))
}
