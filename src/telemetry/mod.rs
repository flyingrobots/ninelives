//! Telemetry and observability for Nine Lives policies.
//!
//! This module provides the event system that enables all policies to emit
//! structured telemetry. Events flow through `TelemetrySink` implementations
//! which can log, aggregate, or forward events to external systems.
//!
//! # Event Types
//!
//! Each policy type emits specific events:
//!
//! - **Retry**: `RetryAttempt`, `RetryExhausted`
//! - **Circuit Breaker**: `CircuitOpened`, `CircuitClosed`, `CircuitHalfOpen`
//! - **Bulkhead**: `BulkheadAcquired`, `BulkheadRejected`
//! - **Timeout**: `TimeoutOccurred`
//! - **All policies**: `RequestSuccess`, `RequestFailure`
//!
//! # Telemetry Sinks
//!
//! The `TelemetrySink` trait defines how events are consumed. It's implemented
//! as a `tower::Service<PolicyEvent>` for composability.

pub mod events;
pub mod sinks;

// Re-export core types for backward compatibility
pub use events::{
    BulkheadEvent, CircuitBreakerEvent, PolicyEvent, RequestOutcome, RetryEvent, TimeoutEvent,
};
pub use sinks::{
    emit_best_effort, ComposedSinkError, FallbackSink, LogSink, MemorySink, MulticastSink,
    NonBlockingSink, NullSink, StreamingSink, TelemetrySink,
};

#[cfg(test)]
mod tests {
    // Note: Tests have been moved to submodules (events.rs and sinks.rs)
}