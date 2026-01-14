use async_trait::async_trait;
use worker::kv::KvStore;

use crate::error::{CroLensError, Result};

#[async_trait(?Send)]
pub trait RateLimitStore {
    async fn get_text(&self, key: &str) -> Result<Option<String>>;
    async fn put_text_with_ttl(&self, key: &str, value: String, ttl_secs: u64) -> Result<()>;
}

#[async_trait(?Send)]
impl RateLimitStore for KvStore {
    async fn get_text(&self, key: &str) -> Result<Option<String>> {
        self.get(key)
            .text()
            .await
            .map_err(|err| CroLensError::KvError(err.to_string()))
    }

    async fn put_text_with_ttl(&self, key: &str, value: String, ttl_secs: u64) -> Result<()> {
        self.put(key, value)
            .map_err(|err| CroLensError::KvError(err.to_string()))?
            .expiration_ttl(ttl_secs)
            .execute()
            .await
            .map_err(|err| CroLensError::KvError(err.to_string()))?;
        Ok(())
    }
}

pub async fn check_rate_limit<S: RateLimitStore>(
    kv: &S,
    key: &str,
    limit: u32,
    window_secs: u64,
) -> Result<bool> {
    if limit == 0 || window_secs == 0 {
        return Ok(true);
    }

    let current = kv.get_text(key).await?;

    let count = current
        .as_deref()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    if count >= limit {
        return Ok(false);
    }

    kv.put_text_with_ttl(key, (count + 1).to_string(), window_secs)
        .await?;

    Ok(true)
}
