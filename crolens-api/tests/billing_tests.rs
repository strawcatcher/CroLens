mod support;

use std::sync::Arc;

use crolens_api::error::CroLensError;
use crolens_api::gateway::auth::ApiKeyRecord;
use crolens_api::gateway::billing::deduct_credit_with_store;
use futures_util::future::join_all;

use support::MemoryApiKeyStore;

#[tokio::test]
async fn test_deduct_credit_success() {
    let store = MemoryApiKeyStore::new(50);
    let api_key = "cl_sk_test_billing_ok_001";

    store
        .set_api_key(ApiKeyRecord {
            api_key: api_key.to_string(),
            tier: "pro".to_string(),
            credits: 2,
            is_active: true,
        })
        .await;

    let remaining = deduct_credit_with_store(&store, api_key)
        .await
        .expect("deduction should succeed");
    assert_eq!(remaining, 1);
}

#[tokio::test]
async fn test_deduct_credit_insufficient() {
    let store = MemoryApiKeyStore::new(50);
    let api_key = "cl_sk_test_billing_zero_001";

    store
        .set_api_key(ApiKeyRecord {
            api_key: api_key.to_string(),
            tier: "pro".to_string(),
            credits: 0,
            is_active: true,
        })
        .await;

    let err = deduct_credit_with_store(&store, api_key)
        .await
        .expect_err("expected payment required");
    assert!(matches!(err, CroLensError::PaymentRequired { .. }));
}

#[tokio::test]
async fn test_atomic_deduction() {
    let store = Arc::new(MemoryApiKeyStore::new(50));
    let api_key = "cl_sk_test_billing_atomic_001";

    store
        .set_api_key(ApiKeyRecord {
            api_key: api_key.to_string(),
            tier: "pro".to_string(),
            credits: 10,
            is_active: true,
        })
        .await;

    let futures = (0..20).map(|_| {
        let store = Arc::clone(&store);
        let api_key = api_key.to_string();
        async move { deduct_credit_with_store(&*store, &api_key).await }
    });

    let mut success = 0;
    let mut failures = 0;
    for out in join_all(futures).await {
        match out {
            Ok(_) => success += 1,
            Err(CroLensError::PaymentRequired { .. }) => failures += 1,
            Err(err) => panic!("unexpected error: {err}"),
        }
    }

    assert_eq!(success, 10);
    assert_eq!(failures, 10);

    let final_record = store
        .get_api_key(api_key)
        .await
        .expect("api key must exist");
    assert_eq!(final_record.credits, 0);
}
