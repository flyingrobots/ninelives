use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_jsonl::JsonlSink;
use tempfile::NamedTempFile;
use tower_service::Service;

#[tokio::test]
async fn writes_json_lines() {
    let temp_file = NamedTempFile::new().expect("failed to create temp file");
    let path = temp_file.path().to_path_buf();
    
    let mut sink = JsonlSink::new(path.to_string_lossy().to_string());

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.expect("Failed to call JsonlSink");

    let contents = std::fs::read_to_string(&path).expect("failed to read temp file");
    let first_line = contents.lines().next().expect("temp file is empty");
    let val: serde_json::Value = serde_json::from_str(first_line).expect("failed to parse JSON from temp file");
    assert_eq!(val["kind"], "retry_attempt");

    // NamedTempFile automatically cleans up when it goes out of scope
}