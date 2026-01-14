mod support;

use crolens_api::gateway::ratelimit::check_rate_limit;

use support::MemoryRateLimitStore;

#[tokio::test]
async fn test_under_limit() {
    let store = MemoryRateLimitStore::new();
    let allowed = check_rate_limit(&store, "rl:test:under", 2, 60)
        .await
        .expect("rate limit check");
    assert!(allowed);
}

#[tokio::test]
async fn test_at_limit() {
    let store = MemoryRateLimitStore::new();

    assert!(check_rate_limit(&store, "rl:test:at", 2, 60).await.unwrap());
    assert!(check_rate_limit(&store, "rl:test:at", 2, 60).await.unwrap());
    assert!(!check_rate_limit(&store, "rl:test:at", 2, 60).await.unwrap());
}
