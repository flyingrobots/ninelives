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

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event.clone())
        .await
        .expect(&format!("Failed to send event to EtcdSink: {:?}", event));

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

    // At minimum, verify the event type and basic structure
    assert!(
        value_str.contains("retry_attempt"),
        "expected event to contain 'retry_attempt', got: {}",
        value_str
    );
    assert!(
        value_str.contains("50") && value_str.contains("attempt"),
        "expected event to contain attempt/delay info, got: {}",
        value_str
    );

    // Cleanup
    client
        .delete(prefix.as_str(), Some(etcd_client::DeleteOptions::new().with_prefix()))
        .await
        .expect("cleanup failed");
}
