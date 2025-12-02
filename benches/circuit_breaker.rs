use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ninelives::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerLayer};

use std::time::Duration;
use tower::{Service, ServiceBuilder};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use futures::future::Ready;

// A simple service that just returns its input.
// Used to chain layers for benchmarking.
#[derive(Clone)]
struct EchoService;

impl Service<&'static str> for EchoService {
    type Response = &'static str;
    type Error = std::io::Error;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: &'static str) -> Self::Future {
        futures::future::ready(Ok(req))
    }
}

// A service that always fails.
#[derive(Clone)]
struct FailingService {
    calls: Arc<AtomicUsize>,
}

impl FailingService {
    fn new() -> Self {
        Self { calls: Arc::new(AtomicUsize::new(0)) }
    }
}

impl Service<&'static str> for FailingService {
    type Response = &'static str;
    type Error = std::io::Error;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: &'static str) -> Self::Future {
        self.calls.fetch_add(1, Ordering::Relaxed);
        futures::future::ready(Err(std::io::Error::new(std::io::ErrorKind::Other, "boom")))
    }
}


fn circuit_breaker_throughput_success(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(10)
        .recovery_timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let layer = CircuitBreakerLayer::new(config).unwrap();
    let svc = ServiceBuilder::new().layer(layer).service(EchoService);

    c.bench_function("circuit_breaker_success_100k_rps", |b| {
        b.to_async(&rt).iter(|| async {
            // Service is cloned for each iteration, which should create new Arc for state.
            // This is important for accurate allocation measurement.
            let mut local_svc = svc.clone();
            let _ = black_box(local_svc.call(black_box("request"))).await;
        });
    });
}

fn circuit_breaker_throughput_failure(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(1) // Open on first failure
        .recovery_timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let layer = CircuitBreakerLayer::new(config).unwrap();
    let svc = ServiceBuilder::new().layer(layer).service(FailingService::new());

    c.bench_function("circuit_breaker_failure_100k_rps", |b| {
        b.to_async(&rt).iter(|| async {
            let mut local_svc = svc.clone();
            let _ = black_box(local_svc.call(black_box("request"))).await;
        });
    });
}


criterion_group!(benches, circuit_breaker_throughput_success, circuit_breaker_throughput_failure);
criterion_main!(benches);