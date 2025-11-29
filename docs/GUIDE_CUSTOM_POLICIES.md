# Building Custom Policies with `ninelives::Policy`

This guide shows how to create your own Tower layer/service and compose it with the Nine Lives algebra (`+`, `|`, `&`). We’ll implement a minimal `RateLimitLayer` and use it alongside an existing `CircuitBreaker`.

## 1) Implement a Tower Service + Layer

```rust
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::task::{Context, Poll, Waker};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct RateLimitLayer {
    limit: usize,
}

impl RateLimitLayer {
    pub fn new(limit: usize) -> Self {
        Self { limit }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    in_flight: Arc<AtomicUsize>,
    waiters: Arc<Mutex<VecDeque<Waker>>>,
    limit: usize,
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            in_flight: Arc::new(AtomicUsize::new(0)),
            waiters: Arc::new(Mutex::new(VecDeque::new())),
            limit: self.limit,
        }
    }
}

impl<S, Request> Service<Request> for RateLimitService<S>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.in_flight.load(Ordering::SeqCst) >= self.limit {
            if let Ok(mut waiters) = self.waiters.lock() {
                waiters.push_back(cx.waker().clone());
            }
            return Poll::Pending;
        }
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut inner = self.inner.clone();
        let in_flight = self.in_flight.clone();
        let waiters = Arc::clone(&self.waiters);
        in_flight.fetch_add(1, Ordering::SeqCst);

        Box::pin(async move {
            let result = inner.call(req).await;
            in_flight.fetch_sub(1, Ordering::SeqCst);
            if let Ok(mut waiters) = waiters.lock() {
                while let Some(w) = waiters.pop_front() {
                    w.wake();
                }
            }
            result
        })
    }
}
```

## 2) Wrap it with `Policy`

```rust
use ninelives::Policy;
use std::time::Duration;
use tower::{ServiceBuilder, ServiceExt};

let rate_limit = Policy(RateLimitLayer::new(100)); // allow 100 in-flight
let breaker = Policy(ninelives::CircuitBreakerLayer::new(
    ninelives::CircuitBreakerConfig::builder()
        .failure_threshold(3)
        .recovery_timeout(Duration::from_secs(5))
        .half_open_limit(1)
        .build()?,
)?);

// Algebra: try rate-limited path, fall back to breaker-protected path
let policy = rate_limit | breaker;

let mut svc = ServiceBuilder::new()
    .layer(policy)
    .service_fn(|req: &'static str| async move {
        Ok::<_, std::io::Error>(format!(\"ok: {req}\"))
    });

let resp = svc.ready().await?.call(\"hello\").await?;
```

## 3) Tips for Custom Policies

- Keep `Request` bounds aligned with the algebra (`Clone + Send + 'static` common case).
- If your layer needs state, wrap it in `Arc` so cloned services share counters safely.
- Emit tracing spans/logs for visibility. Nine Lives does **not** propagate tracing span
  context for you; if you need cross-policy span propagation, attach or re-enter spans in
  your own instrumentation (or carry context on the request) before calling downstream
  services.
- For async locks, prefer `tokio::sync` primitives to avoid blocking the runtime.

That’s it—drop your layer into `Policy(...)` and compose with `+` (wrap), `|` (fallback), or `&` (race) like any built-in policy. Happy building!`
