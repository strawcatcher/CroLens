#![allow(dead_code)]

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::Mutex;

use crolens_api::error::Result;
use crolens_api::gateway::auth::ApiKeyRecord;
use crolens_api::gateway::ratelimit::RateLimitStore;
use crolens_api::gateway::store::ApiKeyStore;

#[derive(Default)]
pub struct MemoryApiKeyStore {
    keys: Mutex<HashMap<String, ApiKeyRecord>>,
    free_daily_limit: i64,
}

impl MemoryApiKeyStore {
    pub fn new(free_daily_limit: i64) -> Self {
        Self {
            keys: Mutex::new(HashMap::new()),
            free_daily_limit,
        }
    }

    pub async fn set_api_key(&self, record: ApiKeyRecord) {
        let mut keys = self.keys.lock().await;
        keys.insert(record.api_key.clone(), record);
    }

    pub async fn get_api_key(&self, api_key: &str) -> Option<ApiKeyRecord> {
        let keys = self.keys.lock().await;
        keys.get(api_key).cloned()
    }
}

#[async_trait(?Send)]
impl ApiKeyStore for MemoryApiKeyStore {
    async fn fetch_api_key(&self, api_key: &str) -> Result<Option<ApiKeyRecord>> {
        Ok(self.get_api_key(api_key).await)
    }

    async fn insert_api_key_if_missing(
        &self,
        api_key: &str,
        _owner_address: Option<&str>,
        tier: &str,
        credits: i64,
        is_active: bool,
    ) -> Result<()> {
        let mut keys = self.keys.lock().await;
        keys.entry(api_key.to_string())
            .or_insert_with(|| ApiKeyRecord {
                api_key: api_key.to_string(),
                tier: tier.to_string(),
                credits,
                is_active,
            });
        Ok(())
    }

    async fn load_free_daily_limit(&self) -> Result<i64> {
        Ok(self.free_daily_limit)
    }

    async fn deduct_credit_if_possible(&self, api_key: &str) -> Result<Option<i64>> {
        let mut keys = self.keys.lock().await;
        let Some(record) = keys.get_mut(api_key) else {
            return Ok(None);
        };
        if !record.is_active || record.credits <= 0 {
            return Ok(None);
        }
        record.credits -= 1;
        Ok(Some(record.credits))
    }
}

#[derive(Default)]
pub struct MemoryRateLimitStore {
    values: Mutex<HashMap<String, String>>,
}

impl MemoryRateLimitStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait(?Send)]
impl RateLimitStore for MemoryRateLimitStore {
    async fn get_text(&self, key: &str) -> Result<Option<String>> {
        let values = self.values.lock().await;
        Ok(values.get(key).cloned())
    }

    async fn put_text_with_ttl(&self, key: &str, value: String, _ttl_secs: u64) -> Result<()> {
        let mut values = self.values.lock().await;
        values.insert(key.to_string(), value);
        Ok(())
    }
}
