use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_jsonl::JsonlSink;
use tempfile::NamedTempFile;
use tower_service::Service;
use tokio::time::timeout;
use std::time::Duration;

#[tokio::test]
async fn writes_json_lines() {
    let temp_file = NamedTempFile::new().expect("failed to create temp file");
    let path = temp_file.path().to_path_buf();
    
    let mut sink = JsonlSink::new(path.clone());

    let attempt = 1;
    let delay_ms = 50;
    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt,
        delay: Duration::from_millis(delay_ms),
    });
    timeout(Duration::from_secs(5), sink.call(event))
        .await
        .expect("JsonlSink call timed out")
        .expect("Failed to call JsonlSink");

    let contents = std::fs::read_to_string(&path).expect("failed to read temp file");
    let first_line = contents.lines().next().expect("temp file is empty");
    let val: serde_json::Value = serde_json::from_str(first_line).expect("failed to parse JSON from temp file");
    assert_eq!(val["kind"], "retry_attempt");
    assert_eq!(val["attempt"], attempt);
    assert_eq!(val["delay_ms"], delay_ms);
}
