#![cfg(feature = "bench-telemetry")]
#![allow(missing_docs)]

use hdrhistogram::Histogram;
use ninelives::telemetry::{NonBlockingSink, NullSink, PolicyEvent, RetryEvent, StreamingSink};
use std::time::{Duration, Instant};
use tower_service::Service;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn telemetry_overhead_baseline() {
    run_bench(NullSink, 100_000, 4, Duration::from_micros(200)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn telemetry_overhead_nonblocking_log() {
    let raw = NullSink; // exercises NonBlockingSink channel path (no actual slow operations)
    let sink = NonBlockingSink::with_capacity(raw, 1024);
    run_bench(sink.clone(), 50_000, 4, Duration::from_micros(500)).await;
    let dropped = sink.dropped();
    println!("NonBlockingSink dropped {} events", dropped);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn telemetry_overhead_streaming_drop_visibility() {
    let sink = StreamingSink::new(64);
    let mut handles = Vec::new();
    for _ in 0..4 {
        let mut clone = sink.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..20_000 {
                // Intentionally ignore result here to exercise drop behavior
                // (do not `.expect()` or assert, otherwise the test's purpose breaks).
                let _ = clone.call(sample_event()).await;
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    // Should drop some when overdriven; verify the counter works.
    // With 4 tasks sending 20,000 events each (80,000 total) into a channel
    // with capacity 64 and no consumer, drops are guaranteed.
    let dropped = sink.dropped_count();
    assert!(dropped > 0, "Expected drops under load, got {}", dropped);
}

async fn run_bench<S>(sink: S, iter: usize, concurrency: usize, p99_budget: Duration)
where
    S: Service<PolicyEvent> + Clone + Send + 'static,
    S::Future: Send,
    <S as Service<PolicyEvent>>::Error: std::fmt::Debug,
{
    assert!(concurrency != 0, "concurrency must be non-zero");

    let mut hist: Histogram<u64> = Histogram::new(3).unwrap();
    let mut tasks = Vec::new();

    let base_iter = iter / concurrency;
    let remainder = iter % concurrency;

    for i in 0..concurrency {
        let mut s = sink.clone();
        let count = if i < remainder { base_iter + 1 } else { base_iter };

        tasks.push(tokio::spawn(async move {
            let mut h = Histogram::new(3).unwrap();
            for _ in 0..count {
                let start = Instant::now();
                s.call(sample_event()).await.expect("sending sample_event failed");
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

fn sample_event() -> PolicyEvent {
    // Non-zero delay keeps retry semantics realistic and avoids zero-duration edge cases.
    PolicyEvent::Retry(RetryEvent::Attempt { attempt: 1, delay: Duration::from_millis(10) })
}
