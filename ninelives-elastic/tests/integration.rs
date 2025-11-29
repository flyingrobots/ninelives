use elasticsearch::{Elasticsearch, SearchParts};
use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_elastic::ElasticSink;
use serde_json::json;
use tokio::runtime::Handle;
use tower::ServiceExt;
use tower_service::Service;
use uuid::Uuid; // Added for unique index generation

struct Cleanup {
    client: Elasticsearch,
    index: String,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let client = self.client.clone();
        let index = self.index.clone();
        let handle = Handle::current();
        let _ = handle.block_on(async move {
            let _ = client
                .indices()
                .delete(elasticsearch::indices::IndicesDeleteParts::Index(&[&index]))
                .send()
                .await;
        });
    }
}

// Requires Elasticsearch running. If NINE_LIVES_TEST_ELASTIC_URL is unset, the test skips.
#[tokio::test]
async fn indexes_policy_events() {
    let url = match std::env::var("NINE_LIVES_TEST_ELASTIC_URL") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skipping: set NINE_LIVES_TEST_ELASTIC_URL (e.g. http://127.0.0.1:9200)");
            return;
        }
    };
    let index = format!("policy-events-{}", Uuid::new_v4()); // Use a unique index
    let transport =
        elasticsearch::http::transport::Transport::single_node(&url).expect("transport");
    let client = Elasticsearch::new(transport);
    let mut sink = ElasticSink::new(client.clone(), index.clone());
    let _guard = Cleanup { client: client.clone(), index: index.clone() };

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.ready().await.expect("sink ready");
    sink.call(event).await.expect("failed to sink policy event to Elasticsearch");

    // refresh index then search
    client
        .indices()
        .refresh(elasticsearch::indices::IndicesRefreshParts::Index(&[&index])) // Use the unique index
        .send()
        .await
        .expect("Failed to refresh index after event ingestion");

    let res = client
        .search(SearchParts::Index(&[&index])) // Use the unique index
        .body(json!({"query": {"match_all": {}}}))
        .send()
        .await
        .expect("failed to execute search query")
        .json::<serde_json::Value>()
        .await
        .expect("failed to parse search response JSON");

    let hits = res["hits"]["hits"]
        .as_array()
        .expect("expected 'hits.hits' to be an array in search response")
        .clone();
    assert_eq!(hits.len(), 1, "expected exactly one indexed event, found {}", hits.len());

    let source = &hits[0]["_source"];
    assert_eq!(source["kind"], "retry_attempt", "kind mismatch");
    assert_eq!(source["attempt"], 1, "attempt mismatch");
    assert_eq!(source["delay_ms"], 50, "delay_ms mismatch");
}
