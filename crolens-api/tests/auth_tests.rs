mod support;

use crolens_api::error::CroLensError;
use crolens_api::gateway::auth::{ensure_api_key_with_store, ApiKeyRecord};

use support::MemoryApiKeyStore;

#[tokio::test]
async fn test_valid_api_key() {
    let store = MemoryApiKeyStore::new(50);
    let api_key = "cl_sk_test_valid_001";

    let record = ensure_api_key_with_store(&store, api_key, None)
        .await
        .expect("api key should be accepted");

    assert_eq!(record.api_key, api_key);
    assert_eq!(record.tier, "free");
    assert_eq!(record.credits, 50);
    assert!(record.is_active);
}

#[tokio::test]
async fn test_invalid_api_key() {
    let store = MemoryApiKeyStore::new(50);

    let err = ensure_api_key_with_store(&store, "not_a_key", None)
        .await
        .expect_err("expected unauthorized");
    assert!(matches!(err, CroLensError::Unauthorized(_)));
}

#[tokio::test]
async fn test_inactive_api_key() {
    let store = MemoryApiKeyStore::new(50);
    let api_key = "cl_sk_test_inactive_001";

    store
        .set_api_key(ApiKeyRecord {
            api_key: api_key.to_string(),
            tier: "free".to_string(),
            credits: 50,
            is_active: false,
        })
        .await;

    let err = ensure_api_key_with_store(&store, api_key, None)
        .await
        .expect_err("expected unauthorized");
    assert!(matches!(err, CroLensError::Unauthorized(_)));
}
