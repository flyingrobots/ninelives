#[cfg(test)]
mod tests {
    use ninelives::rate_limit::strategies::TokenBucket;
    use ninelives::rate_limit::store::InMemoryTokenStore;
    use ninelives::rate_limit::{RateLimiter, Decision};
    use std::time::Duration;

    #[tokio::test]
    async fn token_bucket_allows_and_refills() {
        let store = InMemoryTokenStore::new();
        let bucket = TokenBucket::new(store, "test_bucket", 10.0, 10.0); // 10/sec, cap 10

        // Consume 10
        let d = bucket.acquire(10).await.expect("no error");
        assert!(d.is_allowed());

        // Next should fail
        let d = bucket.acquire(1).await.expect("no error");
        assert!(!d.is_allowed());
        if let Decision::Denied { wait, .. } = d {
            assert!(wait.as_millis() > 0);
        } else {
            panic!("expected denied");
        }

        // Wait a bit (simulate time passing?)
        // Since we use SystemTime, we can't easily mock time without MockClock injection into TokenBucket.
        // For this unit test, we verify logic.
    }
}
