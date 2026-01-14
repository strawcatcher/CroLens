use serde_json::Value;
use worker::d1::D1Type;
use worker::D1Database;

use crate::error::{CroLensError, Result};
use crate::gateway::auth::ApiKeyRecord;
use crate::gateway::store::ApiKeyStore;
use crate::gateway::D1ApiKeyStore;
use crate::infra;

pub async fn deduct_credit_with_store<S: ApiKeyStore>(store: &S, api_key: &str) -> Result<i64> {
    let remaining = store.deduct_credit_if_possible(api_key.trim()).await?;
    remaining.ok_or_else(|| CroLensError::payment_required(None))
}

pub async fn deduct_credit(db: &D1Database, api_key: &str) -> Result<i64> {
    let store = D1ApiKeyStore::new(db);
    deduct_credit_with_store(&store, api_key).await
}

pub async fn grant_credits(
    db: &D1Database,
    api_key: &str,
    owner_address: Option<&str>,
    credits: i64,
    tier: &str,
) -> Result<ApiKeyRecord> {
    let api_key_arg = D1Type::Text(api_key);
    let owner_arg = match owner_address {
        Some(v) if !v.trim().is_empty() => D1Type::Text(v),
        _ => D1Type::Null,
    };
    let tier_arg = D1Type::Text(tier);

    let statement = db
        .prepare(
            "INSERT INTO api_keys (api_key, owner_address, tier, credits, daily_used) \
             VALUES (?1, ?2, ?3, 0, 0) \
             ON CONFLICT(api_key) DO NOTHING",
        )
        .bind_refs([&api_key_arg, &owner_arg, &tier_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    infra::db::run("grant_credits_upsert", statement.run()).await?;

    let credits_arg = D1Type::Integer(credits.clamp(0, i32::MAX as i64) as i32);
    let statement = db
        .prepare(
            "UPDATE api_keys \
             SET credits = credits + ?1, tier = ?2, owner_address = COALESCE(owner_address, ?3) \
             WHERE api_key = ?4 \
             RETURNING api_key, tier, credits, is_active",
        )
        .bind_refs([&credits_arg, &tier_arg, &owner_arg, &api_key_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let result = infra::db::run("grant_credits_update", statement.all()).await?;
    let rows: Vec<Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let Some(row) = rows.first() else {
        return Err(CroLensError::DbError("Failed to grant credits".to_string()));
    };

    let api_key = row
        .get("api_key")
        .and_then(|v| v.as_str())
        .unwrap_or(api_key)
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

    Ok(ApiKeyRecord {
        api_key,
        tier,
        credits,
        is_active,
    })
}
