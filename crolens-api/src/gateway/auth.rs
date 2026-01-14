use worker::D1Database;

use crate::error::{CroLensError, Result};
use crate::gateway::store::ApiKeyStore;
use crate::gateway::D1ApiKeyStore;

#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub api_key: String,
    pub tier: String,
    pub credits: i64,
    pub is_active: bool,
}

pub async fn lookup_api_key(db: &D1Database, api_key: &str) -> Result<Option<ApiKeyRecord>> {
    let store = D1ApiKeyStore::new(db);
    store.fetch_api_key(api_key.trim()).await
}

pub fn validate_api_key_format(api_key: &str) -> Result<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(CroLensError::unauthorized("API key is empty".to_string()));
    }

    let lower = trimmed.to_lowercase();
    if !lower.starts_with("cl_sk_") {
        return Err(CroLensError::unauthorized(
            "API key must start with cl_sk_".to_string(),
        ));
    }

    let suffix = &trimmed["cl_sk_".len()..];
    if suffix.is_empty() {
        return Err(CroLensError::unauthorized(
            "API key suffix is empty".to_string(),
        ));
    }

    let ok = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !ok {
        return Err(CroLensError::unauthorized(
            "API key contains invalid characters".to_string(),
        ));
    }

    Ok(())
}

pub async fn ensure_api_key(
    db: &D1Database,
    api_key: &str,
    owner_address: Option<&str>,
) -> Result<ApiKeyRecord> {
    let store = D1ApiKeyStore::new(db);
    ensure_api_key_with_store(&store, api_key, owner_address).await
}

pub async fn ensure_api_key_with_store<S: ApiKeyStore>(
    store: &S,
    api_key: &str,
    owner_address: Option<&str>,
) -> Result<ApiKeyRecord> {
    let trimmed = api_key.trim();
    validate_api_key_format(trimmed)?;

    if let Some(record) = store.fetch_api_key(trimmed).await? {
        if !record.is_active {
            return Err(CroLensError::unauthorized(
                "API key is inactive".to_string(),
            ));
        }
        return Ok(record);
    }

    let default_credits = store.load_free_daily_limit().await?;
    store
        .insert_api_key_if_missing(trimmed, owner_address, "free", default_credits, true)
        .await?;

    let record = store
        .fetch_api_key(trimmed)
        .await?
        .ok_or_else(|| CroLensError::DbError("Failed to create api key".to_string()))?;

    if !record.is_active {
        return Err(CroLensError::unauthorized(
            "API key is inactive".to_string(),
        ));
    }

    Ok(record)
}
