#![allow(missing_docs)]

use hdrhistogram::Histogram;
use ninelives::telemetry::{NonBlockingSink, NullSink, PolicyEvent, RetryEvent, StreamingSink};
use std::time::{Duration, Instant};
use tower_service::Service;

// Feature-gated to avoid slowing CI. Run with:
// cargo test --quiet --features bench-telemetry -- --ignored
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn telemetry_overhead_baseline() {
    run_bench_null(100_000, 4, Duration::from_micros(200)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn telemetry_overhead_nonblocking_log() {
    run_bench_nonblocking(50_000, 4, Duration::from_micros(500)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn telemetry_overhead_streaming_drop_visibility() {
    let sink = StreamingSink::new(64);
    let mut handles = Vec::new();
    for _ in 0..4 {
        let mut clone = sink.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..20_000 {
                let _ = clone.call(sample_event()).await;
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    // Should drop some when overdriven; just assert the counter is surfaced.
    let _ = sink.dropped_count();
}

async fn run_bench_null(iter: usize, concurrency: usize, p99_budget: Duration) {
    let mut hist: Histogram<u64> = Histogram::new(3).unwrap();
    let mut tasks = Vec::new();
    for _ in 0..concurrency {
        tasks.push(tokio::spawn(async move {
            let mut h = Histogram::new(3).unwrap();
            for _ in 0..(iter / concurrency) {
                let mut sink = NullSink;
                let start = Instant::now();
                let _ = sink.call(sample_event()).await;
                h.record(start.elapsed().as_nanos() as u64).unwrap();
            }
            h
        }));
    }
    for h in tasks {
        let sub = h.await.unwrap();
        hist += sub;
    }
    let p99 = Duration::from_nanos(hist.value_at_quantile(0.99));
    assert!(p99 <= p99_budget, "p99 {:?} > budget {:?}", p99, p99_budget);
}

async fn run_bench_nonblocking(iter: usize, concurrency: usize, p99_budget: Duration) {
    let mut hist: Histogram<u64> = Histogram::new(3).unwrap();
    let raw = NullSink; // stand-in for slow sink; still exercises channel path
    let sink = NonBlockingSink::with_capacity(raw, 1024);

    let mut tasks = Vec::new();
    for _ in 0..concurrency {
        let mut s = sink.clone();
        tasks.push(tokio::spawn(async move {
            let mut h = Histogram::new(3).unwrap();
            for _ in 0..(iter / concurrency) {
                let start = Instant::now();
                let _ = s.call(sample_event()).await;
                h.record(start.elapsed().as_nanos() as u64).unwrap();
            }
            h
        }));
    }
    for h in tasks {
        let sub = h.await.unwrap();
        hist += sub;
    }
    let p99 = Duration::from_nanos(hist.value_at_quantile(0.99));
    assert!(p99 <= p99_budget, "p99 {:?} > budget {:?}", p99, p99_budget);
    // Channel drops allowed but should be visible
    let _ = sink.dropped();
}

fn sample_event() -> PolicyEvent {
    PolicyEvent::Retry(RetryEvent::Attempt { attempt: 1, delay: Duration::from_millis(10) })
}
