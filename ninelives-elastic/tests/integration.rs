use elasticsearch::{Elasticsearch, SearchParts};
use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_elastic::ElasticSink;
use serde_json::json;
use tower_service::Service;

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
    let index = "policy-events";
    let transport =
        elasticsearch::http::transport::Transport::single_node(&url).expect("transport");
    let client = Elasticsearch::new(transport);
    let mut sink = ElasticSink::new(client.clone(), index);

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.unwrap();

    // refresh index then search
    let _ = client
        .indices()
        .refresh(elasticsearch::indices::IndicesRefreshParts::Index(&[index]))
        .send()
        .await;
    let res = client
        .search(SearchParts::Index(&[index]))
        .body(json!({"query": {"match_all": {}}}))
        .send()
        .await
        .expect("search")
        .json::<serde_json::Value>()
        .await
        .expect("json");
    let hits = res["hits"]["hits"].as_array().cloned().unwrap_or_default();
    assert!(!hits.is_empty(), "expected at least one indexed event");
}
