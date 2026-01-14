use async_trait::async_trait;
use serde_json::Value;
use worker::d1::D1Type;
use worker::D1Database;

use crate::error::{CroLensError, Result};
use crate::gateway::auth::ApiKeyRecord;
use crate::infra;

#[async_trait(?Send)]
pub trait ApiKeyStore {
    async fn fetch_api_key(&self, api_key: &str) -> Result<Option<ApiKeyRecord>>;

    async fn insert_api_key_if_missing(
        &self,
        api_key: &str,
        owner_address: Option<&str>,
        tier: &str,
        credits: i64,
        is_active: bool,
    ) -> Result<()>;

    async fn load_free_daily_limit(&self) -> Result<i64>;

    async fn deduct_credit_if_possible(&self, api_key: &str) -> Result<Option<i64>>;
}

pub struct D1ApiKeyStore<'a> {
    db: &'a D1Database,
}

impl<'a> D1ApiKeyStore<'a> {
    pub fn new(db: &'a D1Database) -> Self {
        Self { db }
    }
}

#[async_trait(?Send)]
impl<'a> ApiKeyStore for D1ApiKeyStore<'a> {
    async fn fetch_api_key(&self, api_key: &str) -> Result<Option<ApiKeyRecord>> {
        let api_key_arg = D1Type::Text(api_key);
        let statement = self
            .db
            .prepare("SELECT api_key, tier, credits, is_active FROM api_keys WHERE api_key = ?1")
            .bind_refs([&api_key_arg])
            .map_err(|err| CroLensError::DbError(err.to_string()))?;

        let result = infra::db::run("fetch_api_key", statement.all()).await;
        let result = match result {
            Ok(v) => v,
            Err(CroLensError::DbError(msg))
                if msg.contains("no such column") && msg.contains("is_active") =>
            {
                let statement = self
                    .db
                    .prepare("SELECT api_key, tier, credits FROM api_keys WHERE api_key = ?1")
                    .bind_refs([&api_key_arg])
                    .map_err(|err| CroLensError::DbError(err.to_string()))?;
                infra::db::run("fetch_api_key_legacy", statement.all()).await?
            }
            Err(err) => return Err(err),
        };

        let rows: Vec<Value> = result
            .results()
            .map_err(|err| CroLensError::DbError(err.to_string()))?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };

        let api_key = row
            .get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CroLensError::DbError("api_keys.api_key missing".to_string()))?
            .to_string();

        let tier = row
            .get("tier")
            .and_then(|v| v.as_str())
            .unwrap_or("free")
            .to_string();

        let credits = row.get("credits").and_then(|v| v.as_i64()).unwrap_or(0);
        let is_active = row
            .get("is_active")
            .and_then(|v| v.as_i64())
            .map(|v| v != 0)
            .unwrap_or(true);

        Ok(Some(ApiKeyRecord {
            api_key,
            tier,
            credits,
            is_active,
        }))
    }

    async fn insert_api_key_if_missing(
        &self,
        api_key: &str,
        owner_address: Option<&str>,
        tier: &str,
        credits: i64,
        is_active: bool,
    ) -> Result<()> {
        let api_key_arg = D1Type::Text(api_key);
        let owner_arg = match owner_address {
            Some(v) if !v.trim().is_empty() => D1Type::Text(v),
            _ => D1Type::Null,
        };
        let tier_arg = D1Type::Text(tier);
        let credits_arg = D1Type::Integer(credits.clamp(0, i32::MAX as i64) as i32);
        let is_active_arg = D1Type::Integer(if is_active { 1 } else { 0 });

        let statement = self
            .db
            .prepare(
                "INSERT INTO api_keys (api_key, owner_address, tier, credits, daily_used, is_active) \
                 VALUES (?1, ?2, ?3, ?4, 0, ?5) \
                 ON CONFLICT(api_key) DO NOTHING",
            )
            .bind_refs([&api_key_arg, &owner_arg, &tier_arg, &credits_arg, &is_active_arg])
            .map_err(|err| CroLensError::DbError(err.to_string()))?;

        infra::db::run("insert_api_key_if_missing", statement.run()).await?;
        Ok(())
    }

    async fn load_free_daily_limit(&self) -> Result<i64> {
        let key_arg = D1Type::Text("x402.free_daily_limit");
        let statement = self
            .db
            .prepare("SELECT value FROM system_config WHERE key = ?1 LIMIT 1")
            .bind_refs([&key_arg])
            .map_err(|err| CroLensError::DbError(err.to_string()))?;
        let result = infra::db::run("load_free_daily_limit", statement.all()).await?;
        let rows: Vec<Value> = result
            .results()
            .map_err(|err| CroLensError::DbError(err.to_string()))?;

        Ok(rows
            .first()
            .and_then(|row| row.get("value"))
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(50))
    }

    async fn deduct_credit_if_possible(&self, api_key: &str) -> Result<Option<i64>> {
        let api_key_arg = D1Type::Text(api_key);
        let statement = self
            .db
            .prepare(
                "UPDATE api_keys \
                 SET credits = credits - 1, daily_used = daily_used + 1 \
                 WHERE api_key = ?1 AND credits > 0 AND is_active = 1 \
                 RETURNING credits",
            )
            .bind_refs([&api_key_arg])
            .map_err(|err| CroLensError::DbError(err.to_string()))?;

        let result = infra::db::run("deduct_credit_if_possible", statement.all()).await;
        let result = match result {
            Ok(v) => v,
            Err(CroLensError::DbError(msg))
                if msg.contains("no such column") && msg.contains("is_active") =>
            {
                let statement = self
                    .db
                    .prepare(
                        "UPDATE api_keys \
                         SET credits = credits - 1, daily_used = daily_used + 1 \
                         WHERE api_key = ?1 AND credits > 0 \
                         RETURNING credits",
                    )
                    .bind_refs([&api_key_arg])
                    .map_err(|err| CroLensError::DbError(err.to_string()))?;
                infra::db::run("deduct_credit_if_possible_legacy", statement.all()).await?
            }
            Err(err) => return Err(err),
        };

        let rows: Vec<Value> = result
            .results()
            .map_err(|err| CroLensError::DbError(err.to_string()))?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };

        let remaining = row
            .get("credits")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| CroLensError::DbError("api_keys.credits missing".to_string()))?;

        Ok(Some(remaining))
    }
}
