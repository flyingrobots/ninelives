use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_etcd::EtcdSink;
use tower_service::Service;

// Requires etcd running. If NINE_LIVES_TEST_ETCD_ENDPOINT is unset, the test skips.
#[tokio::test]
async fn writes_events_to_etcd() {
    let Some(endpoint) = std::env::var("NINE_LIVES_TEST_ETCD_ENDPOINT").ok() else {
        eprintln!("skipping: set NINE_LIVES_TEST_ETCD_ENDPOINT (e.g. http://127.0.0.1:2379)");
        return;
    };
    let mut client = etcd_client::Client::connect([endpoint.as_str()], None)
        .await
        .unwrap_or_else(|e| panic!("Failed to connect to etcd at '{}': {}", endpoint, e));

    let prefix = format!("policy_events/{}", uuid::Uuid::new_v4());
    let mut sink = EtcdSink::new(prefix.clone(), client.clone()).expect("valid sink");
    struct Cleanup {
        client: etcd_client::Client,
        prefix: String,
    }
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let mut client = self.client.clone();
            let prefix = self.prefix.clone();
            let handle = tokio::runtime::Handle::current();
            let _ = handle.block_on(async move {
                let _ = client
                    .delete(prefix.as_str(), Some(etcd_client::DeleteOptions::new().with_prefix()))
                    .await;
            });
        }
    }
    let _guard = Cleanup { client: client.clone(), prefix: prefix.clone() };

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.expect("Failed to send event to EtcdSink");

    // read back latest key under prefix
    let resp = client
        .get(prefix.as_str(), Some(etcd_client::GetOptions::new().with_prefix()))
        .await
        .unwrap_or_else(|e| panic!("Failed to get prefix '{}': {}", prefix, e));
    let kvs = resp.kvs();
    assert!(!kvs.is_empty(), "expected at least one kv for prefix '{}'", prefix);

    // Verify event content
    assert_eq!(
        kvs.len(),
        1,
        "expected exactly one key-value pair for prefix '{}', found {}",
        prefix,
        kvs.len()
    );

    let kv = &kvs[0];
    let value_str = std::str::from_utf8(kv.value()).expect("value should be valid UTF-8");
    let json: serde_json::Value = serde_json::from_str(value_str).expect("value should be JSON");
    assert_eq!(json["kind"], "retry_attempt", "kind mismatch");
    assert_eq!(json["attempt"], 1, "attempt mismatch");
    assert_eq!(json["delay_ms"], 50, "delay_ms mismatch");
}
