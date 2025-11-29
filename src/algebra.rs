//! Algebraic composition operators for tower layers.
//!
//! This module provides the `Policy` wrapper and operators for composing
//! tower layers using intuitive algebraic syntax:
//!
//! - `Policy(A) + Policy(B)` - Sequential composition (A wraps B)
//! - `Policy(A) | Policy(B)` - Fallback on error (try A, then B)
//! - `Policy(A) & Policy(B)` - Fork-join (try both concurrently, return first success)
//!
//! # Operator Precedence
//!
//! When combining operators, Rust's standard operator precedence applies:
//! - `&` (BitAnd) has higher precedence than `+` (Add)
//! - `+` (Add) has higher precedence than `|` (BitOr)
//!
//! This means: `A | B + C & D` is parsed as `A | (B + (C & D))`.
//!
//! **Example precedence:**
//! ```text
//! Policy(A) | Policy(B) + Policy(C)
//! // Parses as: Policy(A) | (Policy(B) + Policy(C))
//! // Meaning: Try A, fallback to the combined stack B(C(Service))
//! ```
//!
//! For explicit control, use parentheses:
//! ```text
//! (Policy(A) | Policy(B)) + Policy(C)
//! // Meaning: C wraps a fallback between A and B
//! ```
//!
//! # Examples
//!
//! ## Simple Sequential Composition
//!
//! ```
//! use ninelives::prelude::*;
//! use std::time::Duration;
//! use tower_layer::Layer;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Compose two timeout layers (outer has longer timeout)
//! let policy = Policy(TimeoutLayer::new(Duration::from_secs(5))?)
//!            + Policy(TimeoutLayer::new(Duration::from_secs(1))?);
//! // Stack: Timeout5s(Timeout1s(Service))
//! # Ok(())
//! # }
//! ```
//!
//! ## Fallback Strategy
//!
//! ```
//! use ninelives::prelude::*;
//! use std::time::Duration;
//! use tower_layer::Layer;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let fast = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
//! let slow = Policy(TimeoutLayer::new(Duration::from_secs(5))?);
//! let _policy = fast | slow;
//! // Try 100ms timeout, fallback to 5s timeout on failure
//! # Ok(())
//! # }
//! ```
//!
//! ## Complex Nested Composition
//!
//! ```
//! use ninelives::prelude::*;
//! use std::time::Duration;
//! use tower_layer::Layer;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Aggressive: just a fast timeout
//! let aggressive = Policy(TimeoutLayer::new(Duration::from_millis(50))?);
//!
//! // Defensive: nested timeouts for multiple attempts
//! let defensive = Policy(TimeoutLayer::new(Duration::from_secs(10))?)
//!               + Policy(TimeoutLayer::new(Duration::from_secs(5))?);
//!
//! // Try aggressive first, fallback to defensive
//! let _policy = aggressive | defensive;
//! // Due to precedence: Policy(Timeout50ms) | (Policy(Timeout10s) + Policy(Timeout5s))
//! # Ok(())
//! # }
//! ```

use futures::future::{select, Either};
use std::ops::{Add, BitAnd, BitOr};
use tower_layer::Layer;

/// Opt-in wrapper enabling algebraic composition of tower layers.
///
/// The `Policy` wrapper allows layers to be combined using intuitive operators:
/// - `Policy(A) + Policy(B)` - Sequential composition (A wraps B)
/// - `Policy(A) | Policy(B)` - Fallback on error (try A, then B)
/// - `Policy(A) & Policy(B)` - Fork-join (try both, return first success)
///
/// # Examples
///
/// Sequential composition with `+`:
/// ```
/// use ninelives::prelude::*;
/// use std::time::Duration;
/// use tower_layer::Layer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Wrap a service with two timeouts
/// let _policy = Policy(TimeoutLayer::new(Duration::from_secs(5))?)
///            + Policy(TimeoutLayer::new(Duration::from_secs(1))?);
/// # Ok(())
/// # }
/// ```
///
/// Fallback composition with `|`:
/// ```
/// use ninelives::prelude::*;
/// use std::time::Duration;
/// use tower_layer::Layer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Try with aggressive timeout, fallback to longer timeout
/// let fast = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
/// let slow = Policy(TimeoutLayer::new(Duration::from_secs(5))?);
/// let _policy = fast | slow;
/// # Ok(())
/// # }
/// ```
///
/// Fork-join composition with `&`:
/// ```
/// use ninelives::prelude::*;
/// use std::time::Duration;
/// use tower_layer::Layer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Try both strategies concurrently, use whichever succeeds first
/// let cache_a = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
/// let cache_b = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
/// let _policy = cache_a & cache_b;  // "Happy eyeballs" pattern
/// # Ok(())
/// # }
/// ```
///
/// Nested composition:
/// ```
/// use ninelives::prelude::*;
/// use std::time::Duration;
/// use tower_layer::Layer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Try fast path, fallback to defensive path with longer timeouts
/// let fast = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
/// let defensive = Policy(TimeoutLayer::new(Duration::from_secs(10))?)
///               + Policy(TimeoutLayer::new(Duration::from_secs(5))?);
/// let _policy = fast | defensive;
/// # Ok(())
/// # }
/// ```
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

/// Sequential composition layer that applies `inner` first, then `outer`.
///
/// Created by the `+` operator on `Policy<L>` types:
/// `Policy(A) + Policy(B)` produces `Policy<CombinedLayer<A, B>>`.
///
/// The resulting service stack has `A` as the outermost layer wrapping `B`,
/// which wraps the underlying service: `A(B(Service))`.
///
/// # Example
///
/// ```
/// use ninelives::prelude::*;
/// use std::time::Duration;
/// use tower_layer::Layer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Outer timeout wraps inner timeout
/// let _combined = Policy(TimeoutLayer::new(Duration::from_secs(5))?)
///              + Policy(TimeoutLayer::new(Duration::from_secs(1))?);
/// // Stack: Timeout5s(Timeout1s(Service))
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct CombinedLayer<A, B> {
    /// The outer layer (applied second, wraps `inner`)
    pub outer: A,
    /// The inner layer (applied first, wraps the service)
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

/// Fallback composition layer that tries `primary`, falling back to `secondary` on error.
///
/// Created by the `|` operator on `Policy<L>` types:
/// `Policy(A) | Policy(B)` produces `Policy<FallbackLayer<A, B>>`.
///
/// When a request fails through the primary stack, the original request is
/// retried through the secondary stack. This enables graceful degradation
/// and multi-tier resilience strategies.
///
/// # Example
///
/// ```
/// use ninelives::prelude::*;
/// use std::time::Duration;
/// use tower_layer::Layer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Try fast timeout first, fall back to longer timeout on failure
/// let fast = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
/// let slow = Policy(TimeoutLayer::new(Duration::from_secs(5))?);
/// let _fallback = fast | slow;
/// # Ok(())
/// # }
/// ```
///
/// # Complex Example
///
/// ```
/// use ninelives::prelude::*;
/// use std::time::Duration;
/// use tower_layer::Layer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Try aggressive strategy, fallback to defensive strategy
/// let aggressive = Policy(TimeoutLayer::new(Duration::from_millis(50))?);
/// let defensive = Policy(TimeoutLayer::new(Duration::from_secs(10))?)
///               + Policy(TimeoutLayer::new(Duration::from_secs(5))?);
/// let _policy = aggressive | defensive;
/// // First tries 50ms timeout, on failure tries nested 10s+5s timeouts
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct FallbackLayer<A, B> {
    /// The primary layer strategy (tried first)
    pub primary: A,
    /// The secondary layer strategy (fallback on primary failure)
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

/// Tower service that executes primary, falling back to secondary on error.
///
/// This service is created by [`FallbackLayer`] and implements the actual
/// fallback logic at the service level. On any error from the primary service,
/// the original request is retried through the secondary service.
///
/// Both services must have the same `Response` and `Error` types.
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
    S1::Error: Send + 'static + std::fmt::Debug,
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
            (std::task::Poll::Ready(Err(e)), _) => std::task::Poll::Ready(Err(e)),
            (_, std::task::Poll::Ready(Err(e))) => std::task::Poll::Ready(Err(e)),
            (std::task::Poll::Ready(Ok(_)), std::task::Poll::Ready(Ok(_))) => {
                std::task::Poll::Ready(Ok(()))
            }
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
                Err(primary_err) => match secondary.call(req_clone).await {
                    Ok(resp) => Ok(resp),
                    Err(_) => Err(primary_err), // preserve first failure for diagnostics
                },
            }
        })
    }
}

/// Fork-join composition layer that tries both `left` and `right` concurrently.
///
/// Created by the `&` operator on `Policy<L>` types:
/// `Policy(A) & Policy(B)` produces `Policy<ForkJoinLayer<A, B>>`.
///
/// Both services are called concurrently, and the first successful result is returned.
/// If both fail, an error is returned (currently the left error, but this may change).
///
/// This implements the "happy eyeballs" pattern commonly used for IPv4/IPv6 racing,
/// cache racing, or trying multiple backends simultaneously.
///
/// # Example
///
/// ```
/// use ninelives::prelude::*;
/// use std::time::Duration;
/// use tower_layer::Layer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Race two caches - use whichever responds first
/// let cache_a = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
/// let cache_b = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
/// let _policy = cache_a & cache_b;
/// // Tries both concurrently, returns first Ok result
/// # Ok(())
/// # }
/// ```
///
/// # IPv4/IPv6 Happy Eyeballs Example
///
/// ```
/// use ninelives::prelude::*;
/// use std::time::Duration;
/// use tower_layer::Layer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Try IPv4 and IPv6 in parallel
/// let ipv4_path = Policy(TimeoutLayer::new(Duration::from_millis(300))?);
/// let ipv6_path = Policy(TimeoutLayer::new(Duration::from_millis(300))?);
/// let _policy = ipv4_path & ipv6_path;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct ForkJoinLayer<A, B> {
    /// The left strategy (tried concurrently with right)
    pub left: A,
    /// The right strategy (tried concurrently with left)
    pub right: B,
}

impl<L1, L2> BitAnd<Policy<L2>> for Policy<L1> {
    type Output = Policy<ForkJoinLayer<L1, L2>>;
    fn bitand(self, rhs: Policy<L2>) -> Self::Output {
        Policy(ForkJoinLayer { left: self.0, right: rhs.0 })
    }
}

impl<S, A, B> Layer<S> for ForkJoinLayer<A, B>
where
    S: Clone + Send + 'static,
    A: Layer<S>,
    B: Layer<S>,
    A::Service: Send + 'static,
    B::Service: Send + 'static,
{
    type Service = ForkJoinService<A::Service, B::Service>;

    fn layer(&self, service: S) -> Self::Service {
        let left = self.left.layer(service.clone());
        let right = self.right.layer(service);
        ForkJoinService { left, right }
    }
}

/// Tower service that races two services concurrently, returning the first success.
///
/// This service is created by [`ForkJoinLayer`] and implements the actual
/// fork-join logic at the service level. Both services are called concurrently,
/// and the first `Ok` result is returned. If both fail, returns an error.
///
/// The slower service's future is dropped when the first succeeds.
#[derive(Clone, Debug)]
pub struct ForkJoinService<S1, S2> {
    left: S1,
    right: S2,
}

#[derive(Debug)]
pub struct ForkJoinError<E> {
    pub left: Option<E>,
    pub right: Option<E>,
}

impl<S1, S2, Request> tower_service::Service<Request> for ForkJoinService<S1, S2>
where
    Request: Clone + Send + 'static,
    S1: tower_service::Service<Request> + Clone + Send + 'static,
    S1::Future: Send + 'static,
    S1::Response: Send + 'static,
    S1::Error: Send + 'static + std::fmt::Debug,
    S2: tower_service::Service<Request, Response = S1::Response, Error = S1::Error>
        + Clone
        + Send
        + 'static,
    S2::Future: Send + 'static,
    S2::Response: Send + 'static,
    S2::Error: Send + 'static,
{
    type Response = S1::Response;
    type Error = ForkJoinError<S1::Error>;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let left_ready = self.left.poll_ready(cx);
        let right_ready = self.right.poll_ready(cx);
        match (left_ready, right_ready) {
            (std::task::Poll::Ready(Ok(_)), std::task::Poll::Ready(Ok(_))) => {
                std::task::Poll::Ready(Ok(()))
            }
            (std::task::Poll::Ready(Err(e)), _) => {
                std::task::Poll::Ready(Err(ForkJoinError { left: Some(e), right: None }))
            }
            (_, std::task::Poll::Ready(Err(e))) => {
                std::task::Poll::Ready(Err(ForkJoinError { left: None, right: Some(e) }))
            }
            _ => std::task::Poll::Pending,
        }
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut left = self.left.clone();
        let mut right = self.right.clone();
        let req_clone = req.clone();

        Box::pin(async move {
            use futures::pin_mut;

            let left_fut = left.call(req);
            let right_fut = right.call(req_clone);

            pin_mut!(left_fut);
            pin_mut!(right_fut);

            // Race the two futures
            match select(left_fut, right_fut).await {
                Either::Left((Ok(resp), _)) => Ok(resp),
                Either::Right((Ok(resp), _)) => Ok(resp),
                Either::Left((Err(left_err), right_fut)) => {
                    // Left failed, try right
                    match right_fut.await {
                        Ok(resp) => Ok(resp),
                        Err(right_err) => {
                            // Surface both failures for diagnostics while returning deterministic left error.
                            tracing::debug!(
                                left_error = ?left_err,
                                right_error = ?right_err,
                                "ForkJoinService: both paths failed; returning combined error"
                            );
                            Err(ForkJoinError { left: Some(left_err), right: Some(right_err) })
                        } // Both failed, return combined error
                    }
                }
                Either::Right((Err(right_err), left_fut)) => {
                    // Right failed, try left
                    match left_fut.await {
                        Ok(resp) => Ok(resp),
                        Err(left_err) => {
                            tracing::debug!(
                                left_error = ?left_err,
                                right_error = ?right_err,
                                "ForkJoinService: both paths failed; returning combined error"
                            );
                            Err(ForkJoinError { left: Some(left_err), right: Some(right_err) })
                        } // Both failed, return combined error
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TimeoutLayer;
    use futures::task::noop_waker;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use std::time::Duration;
    use tower::ServiceExt;
    use tower_service::Service;

    #[derive(Clone, Debug)]
    struct GateService {
        ready: Arc<AtomicBool>,
        calls: Arc<AtomicUsize>,
    }

    impl GateService {
        fn new() -> Self {
            Self { ready: Arc::new(AtomicBool::new(false)), calls: Arc::new(AtomicUsize::new(0)) }
        }

        fn set_ready(&self, ready: bool) {
            self.ready.store(ready, Ordering::SeqCst);
        }

        #[allow(dead_code)]
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl<Request> tower_service::Service<Request> for GateService
    where
        Request: Clone + Send + 'static,
    {
        type Response = Request;
        type Error = std::io::Error;
        type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if self.ready.load(Ordering::SeqCst) {
                Poll::Ready(Ok(()))
            } else {
                // wake to allow external state change to be observed
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }

        fn call(&mut self, req: Request) -> Self::Future {
            self.calls.fetch_add(1, Ordering::SeqCst);
            futures::future::ready(Ok(req))
        }
    }

    #[derive(Clone, Debug)]
    struct ReadyService;
    impl ReadyService {
        fn new() -> Self {
            Self
        }
    }
    impl<T: Clone + Send + 'static> tower_service::Service<T> for ReadyService {
        type Response = T;
        type Error = std::io::Error;
        type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: T) -> Self::Future {
            futures::future::ready(Ok(req))
        }
    }

    #[derive(Clone, Debug)]
    struct SlowService {
        delay: Duration,
    }
    impl SlowService {
        fn new(delay: Duration) -> Self {
            Self { delay }
        }
    }
    impl tower_service::Service<&'static str> for SlowService {
        type Response = &'static str;
        type Error = std::io::Error;
        type Future =
            Pin<Box<dyn futures::Future<Output = Result<Self::Response, Self::Error>> + Send>>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: &'static str) -> Self::Future {
            let delay = self.delay;
            Box::pin(async move {
                tokio::time::sleep(delay).await;
                Ok(req)
            })
        }
    }

    #[test]
    fn fallback_poll_ready_waits_for_both() {
        let primary = GateService::new();
        let secondary = GateService::new();

        let mut svc = FallbackService { primary: primary.clone(), secondary: secondary.clone() };

        // Primary ready, secondary not ready => still Pending
        primary.set_ready(true);
        secondary.set_ready(false);
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(matches!(Service::<()>::poll_ready(&mut svc, &mut cx), Poll::Pending));

        // Both ready => Ready
        secondary.set_ready(true);
        assert!(matches!(Service::<()>::poll_ready(&mut svc, &mut cx), Poll::Ready(Ok(()))));
    }

    #[tokio::test]
    async fn fallback_returns_primary_error_when_both_fail() {
        #[derive(Clone, Debug)]
        struct ErrSvc;

        impl tower_service::Service<()> for ErrSvc {
            type Response = ();
            type Error = &'static str;
            type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;
            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: ()) -> Self::Future {
                futures::future::ready(Err("primary failed"))
            }
        }

        let mut svc = FallbackService { primary: ErrSvc, secondary: ErrSvc };
        let err = svc.call(()).await.unwrap_err();
        assert_eq!(err, "primary failed");
    }

    #[tokio::test]
    async fn fork_join_returns_left_error_if_both_fail() {
        #[derive(Clone, Debug)]
        struct LeftErr;
        #[derive(Clone, Debug)]
        struct RightErr;

        impl tower_service::Service<()> for LeftErr {
            type Response = ();
            type Error = &'static str;
            type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;
            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: ()) -> Self::Future {
                futures::future::ready(Err("left"))
            }
        }

        impl tower_service::Service<()> for RightErr {
            type Response = ();
            type Error = &'static str;
            type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;
            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: ()) -> Self::Future {
                futures::future::ready(Err("right"))
            }
        }

        let mut svc = ForkJoinService { left: LeftErr, right: RightErr };
        let err = svc.call(()).await.unwrap_err();
        assert_eq!(err.left, Some("left"));
        assert_eq!(err.right, Some("right"));
    }

    #[test]
    fn fork_join_poll_ready_waits_for_both() {
        let left = GateService::new();
        let right = GateService::new();

        let mut svc = ForkJoinService { left: left.clone(), right: right.clone() };

        left.set_ready(true);
        right.set_ready(false);
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(matches!(Service::<()>::poll_ready(&mut svc, &mut cx), Poll::Pending));

        right.set_ready(true);
        assert!(matches!(Service::<()>::poll_ready(&mut svc, &mut cx), Poll::Ready(Ok(()))));
    }

    #[tokio::test]
    async fn fallback_short_circuits_on_success() {
        let mut svc =
            FallbackService { primary: ReadyService::new(), secondary: GateService::new() };
        let res = svc.call("ok").await.unwrap();
        assert_eq!(res, "ok");
    }

    #[tokio::test]
    async fn race_returns_first_success() {
        let slow = SlowService::new(Duration::from_millis(50));
        let fast = ReadyService::new();
        let mut svc = ForkJoinService { left: slow, right: fast };
        let res = svc.call("req").await.unwrap();
        assert_eq!(res, "req");
    }

    #[tokio::test]
    async fn wrap_preserves_ordering() {
        let layer = CombinedLayer {
            outer: TimeoutLayer::new(Duration::from_millis(20)).unwrap(),
            inner: TimeoutLayer::new(Duration::from_secs(1)).unwrap(),
        };
        let mut svc = layer.layer(ReadyService::new());
        let res = Service::call(&mut svc, "wrapped").await.unwrap();
        assert_eq!(res, "wrapped");
    }

    #[tokio::test]
    async fn fallback_layer_accepts_cloneable_service() {
        // Test that fallback layers work with simple services
        #[derive(Clone)]
        struct TestSvc;
        impl tower_service::Service<()> for TestSvc {
            type Response = ();
            type Error = std::io::Error;
            type Future = futures::future::Ready<Result<(), std::io::Error>>;
            fn poll_ready(
                &mut self,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: ()) -> Self::Future {
                futures::future::ready(Ok(()))
            }
        }

        let left = Policy(tower::util::MapRequestLayer::new(|_: ()| ()));
        let right = Policy(tower::util::MapRequestLayer::new(|_: ()| ()));
        let layer = left | right;
        let mut svc = layer.layer(TestSvc);
        assert!(matches!(
            Service::poll_ready(&mut svc, &mut Context::from_waker(&noop_waker())),
            Poll::Ready(Ok(()))
        ));
        svc.call(()).await.unwrap();
    }

    #[tokio::test]
    async fn fork_join_layer_accepts_cloneable_service() {
        // Test that fork-join layers work with simple services
        #[derive(Clone)]
        struct TestSvc;
        impl tower_service::Service<&'static str> for TestSvc {
            type Response = &'static str;
            type Error = std::io::Error;
            type Future = futures::future::Ready<Result<&'static str, std::io::Error>>;
            fn poll_ready(
                &mut self,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Ok(()))
            }
            fn call(&mut self, req: &'static str) -> Self::Future {
                futures::future::ready(Ok(req))
            }
        }

        let fast = Policy(tower::util::MapRequestLayer::new(|req: &'static str| req));
        let slow = Policy(tower::util::MapRequestLayer::new(|req: &'static str| req));
        let layer = fast & slow;
        let mut svc = layer.layer(TestSvc);
        let resp = svc.ready().await.unwrap().call("ok").await.unwrap();
        assert_eq!(resp, "ok");
    }

    #[tokio::test]
    async fn fork_join_returns_both_errors_on_dual_failure() {
        #[derive(Clone)]
        struct FailSvc(&'static str);
        impl tower_service::Service<&'static str> for FailSvc {
            type Response = &'static str;
            type Error = &'static str;
            type Future = futures::future::Ready<Result<&'static str, &'static str>>;
            fn poll_ready(
                &mut self,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: &'static str) -> Self::Future {
                futures::future::ready(Err(self.0))
            }
        }

        let left = Policy(tower::util::MapRequestLayer::new(|req: &'static str| req))
            .layer(FailSvc("left"));
        let right = Policy(tower::util::MapRequestLayer::new(|req: &'static str| req))
            .layer(FailSvc("right"));
        let mut svc = ForkJoinService { left, right };

        let err = svc.call("req").await.unwrap_err();
        assert_eq!(err.left, Some("left"));
        assert_eq!(err.right, Some("right"));
    }
}
