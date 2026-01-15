use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use worker::d1::D1Type;
use worker::kv::KvStore;
use worker::D1Database;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

const TOKENS_CACHE_KEY: &str = "cache:tokens:all";
const TOKENS_CACHE_TTL_SECS: u64 = 600; // 10 分钟

#[derive(Debug, Clone)]
pub struct Token {
    pub address: Address,
    pub symbol: String,
    pub decimals: u8,
    pub is_stablecoin: bool,
}

#[derive(Serialize, Deserialize)]
struct TokenCache {
    address: String,
    symbol: String,
    decimals: u8,
    is_stablecoin: bool,
}

/// 从 KV 缓存获取代币列表，缓存未命中时从 DB 加载
pub async fn list_tokens_cached(db: &D1Database, kv: &KvStore) -> Result<Vec<Token>> {
    // 先尝试从 KV 缓存获取
    if let Ok(Some(cached)) = kv.get(TOKENS_CACHE_KEY).text().await {
        if let Ok(tokens_cache) = serde_json::from_str::<Vec<TokenCache>>(&cached) {
            let mut tokens = Vec::with_capacity(tokens_cache.len());
            for t in tokens_cache {
                if let Ok(addr) = types::parse_address(&t.address) {
                    tokens.push(Token {
                        address: addr,
                        symbol: t.symbol,
                        decimals: t.decimals,
                        is_stablecoin: t.is_stablecoin,
                    });
                }
            }
            if !tokens.is_empty() {
                return Ok(tokens);
            }
        }
    }

    // 缓存未命中，从 DB 加载
    let tokens = list_tokens(db).await?;

    // 写入缓存
    let cache: Vec<TokenCache> = tokens
        .iter()
        .map(|t| TokenCache {
            address: t.address.to_string(),
            symbol: t.symbol.clone(),
            decimals: t.decimals,
            is_stablecoin: t.is_stablecoin,
        })
        .collect();
    if let Ok(json) = serde_json::to_string(&cache) {
        if let Ok(put) = kv.put(TOKENS_CACHE_KEY, json) {
            let _ = put.expiration_ttl(TOKENS_CACHE_TTL_SECS).execute().await;
        }
    }

    Ok(tokens)
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
