//! Algebraic composition operators for tower layers.
//!
//! This module provides the `Policy` wrapper and operators for composing
//! tower layers using intuitive algebraic syntax:
//!
//! - `Policy(A) + Policy(B)` - Sequential composition (A wraps B)
//! - `Policy(A) | Policy(B)` - Fallback on error (try A, then B)
//!
//! # Operator Precedence
//!
//! When combining operators, Rust's standard operator precedence applies:
//! - `+` (Add) has higher precedence than `|` (BitOr)
//!
//! This means: `A | B + C` is parsed as `A | (B + C)`.
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

use std::ops::{Add, BitOr};
use tower_layer::Layer;

/// Opt-in wrapper enabling algebraic composition of tower layers.
///
/// The `Policy` wrapper allows layers to be combined using intuitive operators:
/// - `Policy(A) + Policy(B)` - Sequential composition (A wraps B)
/// - `Policy(A) | Policy(B)` - Fallback on error (try A, then B)
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
