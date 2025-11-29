use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_etcd::EtcdSink;
use tower_service::Service;

// Requires etcd running. If NINE_LIVES_TEST_ETCD_ENDPOINT is unset, the test skips.
#[tokio::test]
async fn writes_events_to_etcd() {
    let endpoint = match std::env::var("NINE_LIVES_TEST_ETCD_ENDPOINT") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skipping: set NINE_LIVES_TEST_ETCD_ENDPOINT (e.g. http://127.0.0.1:2379)");
            return;
        }
    };
    let mut client = etcd_client::Client::connect([endpoint.as_str()], None).await.expect("client");
    let mut sink = EtcdSink::new("policy_events", client.clone());

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.unwrap();

    // read back latest key under prefix
    let resp = client
        .get("policy_events", Some(etcd_client::GetOptions::new().with_prefix()))
        .await
        .expect("get");
    let kvs = resp.kvs();
    assert!(!kvs.is_empty(), "expected at least one kv");
}
