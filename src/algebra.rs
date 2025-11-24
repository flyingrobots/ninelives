use std::ops::{Add, BitOr};
use tower_layer::Layer;

/// Opt-in wrapper enabling algebraic composition of layers.
/// Opt-in wrapper enabling algebraic composition of layers.
#[derive(Clone, Copy, Debug)]
pub struct Policy<L>(pub L);

impl<S, L> Layer<S> for Policy<L>
where
    L: Layer<S>,
{
    type Service = L::Service;
    fn layer(&self, service: S) -> Self::Service {
        self.0.layer(service)
    }
}

/// Sequential composition: apply `inner`, then `outer`.
/// Sequential composition: apply `inner`, then `outer`.
#[derive(Clone, Copy, Debug)]
pub struct CombinedLayer<A, B> {
    pub outer: A,
    pub inner: B,
}

impl<L1, L2> Add<Policy<L2>> for Policy<L1> {
    type Output = Policy<CombinedLayer<L1, L2>>;
    fn add(self, rhs: Policy<L2>) -> Self::Output {
        Policy(CombinedLayer { outer: self.0, inner: rhs.0 })
    }
}

impl<S, A, B> Layer<S> for CombinedLayer<A, B>
where
    B: Layer<S>,
    A: Layer<B::Service>,
{
    type Service = A::Service;
    fn layer(&self, service: S) -> Self::Service {
        self.outer.layer(self.inner.layer(service))
    }
}

/// Fallback composition: try primary; on error, use secondary.
/// Fallback composition: try primary; on error, use secondary.
#[derive(Clone, Debug)]
pub struct FallbackLayer<A, B> {
    pub primary: A,
    pub secondary: B,
}

impl<L1, L2> BitOr<Policy<L2>> for Policy<L1> {
    type Output = Policy<FallbackLayer<L1, L2>>;
    fn bitor(self, rhs: Policy<L2>) -> Self::Output {
        Policy(FallbackLayer { primary: self.0, secondary: rhs.0 })
    }
}

impl<S, A, B> Layer<S> for FallbackLayer<A, B>
where
    S: Clone + Send + 'static,
    A: Layer<S>,
    B: Layer<S>,
    A::Service: Send + 'static,
    B::Service: Send + 'static,
{
    type Service = FallbackService<A::Service, B::Service>;

    fn layer(&self, service: S) -> Self::Service {
        let primary = self.primary.layer(service.clone());
        let secondary = self.secondary.layer(service);
        FallbackService { primary, secondary }
    }
}

/// Service that tries primary, falls back to secondary on error.
/// Service that tries primary first, then falls back to secondary on error.
#[derive(Clone, Debug)]
pub struct FallbackService<S1, S2> {
    primary: S1,
    secondary: S2,
}

impl<S1, S2, Request> tower_service::Service<Request> for FallbackService<S1, S2>
where
    Request: Clone + Send + 'static,
    S1: tower_service::Service<Request> + Clone + Send + 'static,
    S1::Future: Send + 'static,
    S1::Response: Send + 'static,
    S1::Error: Send + 'static,
    S2: tower_service::Service<Request, Response = S1::Response, Error = S1::Error>
        + Clone
        + Send
        + 'static,
    S2::Future: Send + 'static,
    S2::Response: Send + 'static,
    S2::Error: Send + 'static,
{
    type Response = S1::Response;
    type Error = S1::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let primary_ready = self.primary.poll_ready(cx);
        let secondary_ready = self.secondary.poll_ready(cx);
        match (primary_ready, secondary_ready) {
            (std::task::Poll::Ready(Ok(_)), _) => std::task::Poll::Ready(Ok(())),
            (_, std::task::Poll::Ready(Ok(_))) => std::task::Poll::Ready(Ok(())),
            (std::task::Poll::Ready(Err(e)), _) => std::task::Poll::Ready(Err(e)),
            (_, std::task::Poll::Ready(Err(e))) => std::task::Poll::Ready(Err(e)),
            _ => std::task::Poll::Pending,
        }
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut primary = self.primary.clone();
        let mut secondary = self.secondary.clone();
        let req_clone = req.clone();
        Box::pin(async move {
            match primary.call(req).await {
                Ok(resp) => Ok(resp),
                Err(_) => secondary.call(req_clone).await,
            }
        })
    }
}
