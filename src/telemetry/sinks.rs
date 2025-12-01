use super::events::PolicyEvent;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tower::Service;

/// A telemetry sink that consumes policy events.
pub trait TelemetrySink:
    tower::Service<PolicyEvent, Response = (), Error = Self::SinkError> + Clone + Send + 'static
{
    /// The error type for this sink.
    type SinkError: std::error::Error + Send + 'static;
}

/// Best-effort emit helper that honors `poll_ready` and swallows errors.
pub async fn emit_best_effort<S>(sink: S, event: PolicyEvent)
where
    S: tower::Service<PolicyEvent, Response = ()> + Send + Clone + 'static,
    S::Error: std::error::Error + Send + 'static,
    S::Future: Send + 'static,
{
    use tower::ServiceExt;

    if let Ok(mut ready_sink) = sink.ready_oneshot().await {
        let _ = ready_sink.call(event).await;
    }
}

/// A no-op telemetry sink that discards all events.
#[derive(Clone, Debug, Default)]
pub struct NullSink;

impl Service<PolicyEvent> for NullSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _event: PolicyEvent) -> Self::Future {
        Box::pin(async { Ok(()) })
    }
}

impl TelemetrySink for NullSink {
    type SinkError = Infallible;
}

/// A telemetry sink that logs events using the `tracing` crate.
#[derive(Clone, Debug, Default)]
pub struct LogSink;

impl Service<PolicyEvent> for LogSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        tracing::info!(event = %event, "policy_event");
        Box::pin(async { Ok(()) })
    }
}

impl TelemetrySink for LogSink {
    type SinkError = Infallible;
}

/// A telemetry sink that stores events in memory.
#[derive(Clone, Debug)]
pub struct MemorySink {
    events: Arc<Mutex<Vec<PolicyEvent>>>,
    capacity: usize,
    evicted: Arc<AtomicU64>,
}

impl MemorySink {
    pub fn new() -> Self {
        Self::with_capacity(10_000)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            capacity: capacity.max(1),
            evicted: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn unbounded() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            capacity: usize::MAX,
            evicted: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn events(&self) -> Vec<PolicyEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn evicted(&self) -> u64 {
        self.evicted.load(Ordering::Relaxed)
    }
}

impl Default for MemorySink {
    fn default() -> Self {
        Self::new()
    }
}

impl Service<PolicyEvent> for MemorySink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let mut guard = self.events.lock().unwrap();
        if guard.len() >= self.capacity {
            guard.remove(0);
            self.evicted.fetch_add(1, Ordering::Relaxed);
        }
        guard.push(event);
        Box::pin(async { Ok(()) })
    }
}

impl TelemetrySink for MemorySink {
    type SinkError = Infallible;
}

/// A streaming telemetry sink that broadcasts events to multiple subscribers.
#[derive(Clone, Debug)]
pub struct StreamingSink {
    sender: Arc<tokio::sync::broadcast::Sender<PolicyEvent>>,
    dropped: Arc<AtomicU64>,
    last_drop_ns: Arc<AtomicU64>,
}

impl StreamingSink {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(capacity);
        Self {
            sender: Arc::new(sender),
            dropped: Arc::new(AtomicU64::new(0)),
            last_drop_ns: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<PolicyEvent> {
        self.sender.subscribe()
    }

    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn last_drop(&self) -> Option<SystemTime> {
        match self.last_drop_ns.load(Ordering::Relaxed) {
            0 => None,
            ns => UNIX_EPOCH.checked_add(Duration::from_nanos(ns)),
        }
    }
}

impl Service<PolicyEvent> for StreamingSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        if let Err(_e) = self.sender.send(event) {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            self.last_drop_ns.store(
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64,
                Ordering::Relaxed,
            );
        }
        Box::pin(async { Ok(()) })
    }
}

impl TelemetrySink for StreamingSink {
    type SinkError = Infallible;
}

/// Offloads telemetry emission to a bounded channel and worker task.
#[derive(Clone)]
pub struct NonBlockingSink<S> {
    tx: tokio::sync::mpsc::Sender<PolicyEvent>,
    dropped: Arc<AtomicU64>,
    _sink: Arc<tokio::sync::Mutex<S>>,
}

impl<S> NonBlockingSink<S>
where
    S: tower::Service<PolicyEvent, Response = ()> + Send + Clone + 'static,
    S::Error: std::error::Error + Send + 'static,
    S::Future: Send + 'static,
{
    pub fn with_capacity(sink: S, capacity: usize) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);
        let dropped = Arc::new(AtomicU64::new(0));
        let dropped_clone = dropped.clone();
        let sink_arc = Arc::new(tokio::sync::Mutex::new(sink));
        let sink_worker = sink_arc.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                use tower::ServiceExt;
                let mut guard = sink_worker.lock().await;
                if let Ok(ready) = guard.ready().await {
                    let _ = ready.call(event).await;
                }
            }
        });

        Self { tx, dropped: dropped_clone, _sink: sink_arc }
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl<S> tower::Service<PolicyEvent> for NonBlockingSink<S>
where
    S: tower::Service<PolicyEvent, Response = ()> + Send + Clone + 'static,
    S::Error: std::error::Error + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        if self.tx.try_send(event).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        Box::pin(async { Ok(()) })
    }
}

impl<S> TelemetrySink for NonBlockingSink<S>
where
    S: tower::Service<PolicyEvent, Response = ()> + Send + Clone + 'static,
    S::Error: std::error::Error + Send + 'static,
    S::Future: Send + 'static,
{
    type SinkError = Infallible;
}

#[derive(Debug)]
pub struct ComposedSinkError(Box<dyn std::error::Error + Send + Sync>);

impl std::fmt::Display for ComposedSinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "telemetry sink error: {}", self.0)
    }
}

impl std::error::Error for ComposedSinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.0)
    }
}

#[derive(Clone)]
pub struct MulticastSink<A, B> {
    sink_a: A,
    sink_b: B,
}

impl<A, B> MulticastSink<A, B> {
    pub fn new(sink_a: A, sink_b: B) -> Self {
        Self { sink_a, sink_b }
    }
}

impl<A, B> Service<PolicyEvent> for MulticastSink<A, B>
where
    A: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    A::Error: std::error::Error + Send + Sync + 'static,
    A::Future: Send + 'static,
    B: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
    B::Future: Send + 'static,
{
    type Response = ();
    type Error = ComposedSinkError;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let mut sink_a = self.sink_a.clone();
        let mut sink_b = self.sink_b.clone();
        let event_clone = event.clone();

        Box::pin(async move {
            let (res_a, res_b) = tokio::join!(sink_a.call(event), sink_b.call(event_clone));
            res_a.map_err(|e| ComposedSinkError(Box::new(e)))?;
            res_b.map_err(|e| ComposedSinkError(Box::new(e)))?;
            Ok(())
        })
    }
}

impl<A, B> TelemetrySink for MulticastSink<A, B>
where
    A: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    A::Error: std::error::Error + Send + Sync + 'static,
    A::Future: Send + 'static,
    B: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
    B::Future: Send + 'static,
{
    type SinkError = ComposedSinkError;
}

#[derive(Clone)]
pub struct FallbackSink<A, B> {
    primary: A,
    fallback: B,
}

impl<A, B> FallbackSink<A, B> {
    pub fn new(primary: A, fallback: B) -> Self {
        Self { primary, fallback }
    }
}

impl<A, B> Service<PolicyEvent> for FallbackSink<A, B>
where
    A: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    A::Error: std::error::Error + Send + Sync + 'static,
    A::Future: Send + 'static,
    B: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
    B::Future: Send + 'static,
{
    type Response = ();
    type Error = ComposedSinkError;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let mut primary = self.primary.clone();
        let mut fallback = self.fallback.clone();
        let event_clone = event.clone();

        Box::pin(async move {
            match primary.call(event).await {
                Ok(()) => Ok(()),
                Err(_) => {
                    fallback.call(event_clone).await.map_err(|e| ComposedSinkError(Box::new(e)))
                }
            }
        })
    }
}

impl<A, B> TelemetrySink for FallbackSink<A, B>
where
    A: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    A::Error: std::error::Error + Send + Sync + 'static,
    A::Future: Send + 'static,
    B: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
    B::Future: Send + 'static,
{
    type SinkError = ComposedSinkError;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::events::{CircuitBreakerEvent, RetryEvent, TimeoutEvent, BulkheadEvent};
    use std::time::Duration;

    #[tokio::test]
    async fn test_null_sink() {
        let mut sink = NullSink;
        let event = PolicyEvent::Retry(RetryEvent::Attempt {
            attempt: 1,
            delay: Duration::from_millis(100),
        });
        sink.call(event).await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_sink() {
        let mut sink = MemorySink::with_capacity(2);
        assert!(sink.is_empty());
        assert_eq!(sink.len(), 0);

        let event1 = PolicyEvent::Retry(RetryEvent::Attempt {
            attempt: 1,
            delay: Duration::from_millis(100),
        });
        let event2 = PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Opened { failure_count: 5 });
        let event3 =
            PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: Duration::from_secs(1) });

        sink.call(event1.clone()).await.unwrap();
        sink.call(event2.clone()).await.unwrap();
        sink.call(event3.clone()).await.unwrap(); // should evict oldest

        assert_eq!(sink.len(), 2);
        assert!(!sink.is_empty());
        assert_eq!(sink.evicted(), 1);

        let events = sink.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], event2);
        assert_eq!(events[1], event3);

        sink.clear();
        assert!(sink.is_empty());
    }

    #[tokio::test]
    async fn test_streaming_sink_drop_counts() {
        let sink = StreamingSink::new(1);
        let mut tx = sink.clone();

        tx.call(PolicyEvent::Bulkhead(BulkheadEvent::Rejected {
            active_count: 1,
            max_concurrency: 1,
        }))
        .await
        .unwrap();

        assert!(sink.dropped_count() >= 1);
    }

    #[tokio::test]
    async fn test_streaming_sink_last_drop_updates() {
        let sink = StreamingSink::new(1);
        let mut tx = sink.clone();

        tx.call(PolicyEvent::Retry(RetryEvent::Attempt {
            attempt: 1,
            delay: Duration::from_millis(5),
        }))
        .await
        .unwrap();

        assert!(sink.last_drop().is_some());
    }

    #[tokio::test]
    async fn test_streaming_sink_delivers_to_subscriber() {
        let sink = StreamingSink::new(8);
        let mut rx = sink.subscribe();
        let mut tx = sink.clone();

        tx.call(PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: Duration::from_millis(5) }))
            .await
            .unwrap();
        let got = rx.recv().await.expect("message");
        assert!(matches!(got, PolicyEvent::Timeout(_)));
    }

    #[tokio::test]
    async fn test_emit_best_effort_swallows_errors() {
        #[derive(Clone)]
        struct Fails;
        impl TelemetrySink for Fails {
            type SinkError = std::io::Error;
        }
        impl tower::Service<PolicyEvent> for Fails {
            type Response = ();
            type Error = std::io::Error;
            type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>>;
            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: PolicyEvent) -> Self::Future {
                Box::pin(async { Err(std::io::Error::new(std::io::ErrorKind::Other, "fail")) })
            }
        }

        emit_best_effort(
            Fails,
            PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: Duration::from_millis(1) }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_log_sink() {
        let mut sink = LogSink;
        let event =
            PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: Duration::from_secs(1) });
        sink.call(event).await.unwrap();
    }
}
