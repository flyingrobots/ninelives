//! Rate limiting primitives.
//!
//! This module provides the building blocks for rate limiting:
//! - [`RateLimiter`]: The core trait for rate limiting logic.
//! - [`RateLimitLayer`]: Tower middleware that enforces the limit.
//! - [`Decision`]: The result of a rate limit check (Allowed/Denied).
//!
//! # Architecture
//!
//! The system is designed to be modular:
//! - **Middleware**: `RateLimitLayer` wraps your service. It doesn't know *how* limiting works,
//!   only that it should ask a `RateLimiter`.
//! - **Logic**: Implementations like `TokenBucket` (in `strategies` module) handle the math.
//! - **Storage**: `TokenStore` (in `store` module) handles the state, enabling
//!   in-memory or distributed backends (e.g., Redis).

use std::time::Duration;
use std::collections::HashMap;

pub mod middleware;
pub mod store;
pub mod strategies;
pub use middleware::{RateLimitLayer, RateLimitService};

/// The decision returned by a rate limiter.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// The request is allowed to proceed.
    Allowed {
        /// Number of permits remaining after this acquisition.
        /// Useful for `X-RateLimit-Remaining` headers.
        remaining: u32,
        /// Optional metadata (e.g., "reset time", "tier").
        metadata: HashMap<String, String>,
    },
    /// The request is denied.
    Denied {
        /// How long the caller should wait before retrying.
        /// Useful for `Retry-After` headers.
        wait: Duration,
        /// Reason for denial (e.g., "global_limit", "user_limit").
        reason: String,
    },
}

impl Decision {
    /// Helper to check if allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Decision::Allowed { .. })
    }
}

/// Core interface for rate limiting logic.
///
/// This trait allows decoupling the middleware from the implementation (Token Bucket,
/// Leaky Bucket, Fixed Window) and the storage (Memory, Redis).
#[async_trait::async_trait]
pub trait RateLimiter: Send + Sync {
    /// Attempt to acquire the specified number of permits.
    async fn acquire(&self, permits: u32) -> Result<Decision, Box<dyn std::error::Error + Send + Sync>>;
}
