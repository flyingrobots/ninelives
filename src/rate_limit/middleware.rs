use tower_layer::Layer;
use tower_service::Service;
use std::task::{Context, Poll};
use std::future::Future;
use std::pin::Pin;
use crate::ResilienceError;
use crate::rate_limit::{RateLimiter, Decision};
use std::sync::Arc;

/// A layer that enforces rate limits using a [`RateLimiter`].
#[derive(Clone, Debug)]
pub struct RateLimitLayer<L> {
    limiter: Arc<L>,
}

impl<L> RateLimitLayer<L> {
    /// Create a new rate limit layer.
    pub fn new(limiter: L) -> Self {
        Self { limiter: Arc::new(limiter) }
    }
}

impl<S, L> Layer<S> for RateLimitLayer<L>
where
    L: RateLimiter + 'static,
{
    type Service = RateLimitService<S, L>;

    fn layer(&self, service: S) -> Self::Service {
        RateLimitService {
            inner: service,
            limiter: self.limiter.clone(),
        }
    }
}

/// Middleware service that enforces rate limits.
#[derive(Clone, Debug)]
pub struct RateLimitService<S, L> {
    inner: S,
    limiter: Arc<L>,
}

impl<S, L, Req> Service<Req> for RateLimitService<S, L>
where
    S: Service<Req> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + Sync + std::error::Error + 'static,
    L: RateLimiter + Send + Sync + 'static,
    Req: Send + 'static,
{
    type Response = S::Response;
    type Error = ResilienceError<S::Error>;
    // Use BoxFuture for now; can optimize to pin-project later if needed.
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(ResilienceError::Inner)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let limiter = self.limiter.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Acquire 1 permit by default.
            // Future enhancement: Allow extracting cost from request.
            match limiter.acquire(1).await {
                Ok(Decision::Allowed { .. }) => {
                    inner.call(req).await.map_err(ResilienceError::Inner)
                }
                Ok(Decision::Denied { wait, reason: _ }) => {
                    Err(ResilienceError::RateLimited { wait })
                }
                Err(e) => {
                    // Limiter failed (e.g., Redis down).
                    Err(ResilienceError::Infrastructure(e.to_string()))
                }
            }
        })
    }
}
