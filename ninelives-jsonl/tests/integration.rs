use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_jsonl::JsonlSink;
use std::path::PathBuf;
use tower_service::Service;

#[tokio::test]
async fn writes_json_lines() {
    let path = PathBuf::from("/tmp/ninelives-jsonl-test.log");
    // ensure clean
    let _ = std::fs::remove_file(&path);
    let mut sink = JsonlSink::new(path.to_string_lossy().to_string());

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.unwrap();

    let contents = std::fs::read_to_string(&path).expect("file");
    assert!(contents.contains("retry_attempt"));
}
