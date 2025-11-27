//! Circuit breaker implemented as a tower Layer/Service.

use crate::{adaptive::Adaptive, clock::Clock, clock::MonotonicClock, ResilienceError};
use futures::future::BoxFuture;
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tower_layer::Layer;
use tower_service::Service;

const STATE_CLOSED: u8 = 0;
const STATE_OPEN: u8 = 1;
const STATE_HALF_OPEN: u8 = 2;

/// Circuit breaker state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed - requests flow normally
    Closed,
    /// Circuit is open - requests are rejected immediately
    Open,
    /// Circuit is half-open - limited test requests allowed
    HalfOpen,
}

impl CircuitState {
    fn to_u8(self) -> u8 {
        match self {
            CircuitState::Closed => STATE_CLOSED,
            CircuitState::Open => STATE_OPEN,
            CircuitState::HalfOpen => STATE_HALF_OPEN,
        }
    }

    fn from_u8(v: u8) -> CircuitState {
        match v {
            STATE_CLOSED => CircuitState::Closed,
            STATE_OPEN => CircuitState::Open,
            STATE_HALF_OPEN => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }
}

/// Validated configuration for the circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    id: Option<String>,
    failure_threshold: Adaptive<usize>,
    recovery_timeout: Adaptive<Duration>,
    half_open_max_calls: Adaptive<usize>,
}

/// Errors produced when validating breaker configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerError {
    /// Failure threshold must be greater than zero
    InvalidFailureThreshold {
        /// The invalid threshold value provided
        provided: usize,
    },
    /// Recovery timeout duration must be greater than zero
    InvalidRecoveryTimeout(Duration),
    /// Half-open call limit must be greater than zero
    InvalidHalfOpenLimit {
        /// The invalid limit value provided
        provided: usize,
    },
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::InvalidFailureThreshold { provided } => {
                write!(f, "failure_threshold must be > 0 (got {})", provided)
            }
            CircuitBreakerError::InvalidRecoveryTimeout(timeout) => {
                write!(f, "recovery_timeout must be > 0 (got {:?})", timeout)
            }
            CircuitBreakerError::InvalidHalfOpenLimit { provided } => {
                write!(f, "half_open_max_calls must be > 0 (got {})", provided)
            }
        }
    }
}

impl std::error::Error for CircuitBreakerError {}

impl CircuitBreakerConfig {
    /// Create a new circuit breaker configuration with validation.
    ///
    /// # Errors
    ///
    /// Returns error if any parameter is zero or invalid.
    pub fn new(
        failure_threshold: usize,
        recovery_timeout: Duration,
        half_open_max_calls: usize,
    ) -> Result<Self, CircuitBreakerError> {
        let cfg = Self {
            id: None,
            failure_threshold: Adaptive::new(failure_threshold),
            recovery_timeout: Adaptive::new(recovery_timeout),
            half_open_max_calls: Adaptive::new(half_open_max_calls),
        };
        cfg.validate()?;
        Ok(cfg)
    }

    /// Create a disabled circuit breaker (never opens).
    pub fn disabled() -> Self {
        Self {
            id: None,
            failure_threshold: Adaptive::new(usize::MAX),
            recovery_timeout: Adaptive::new(Duration::MAX),
            half_open_max_calls: Adaptive::new(usize::MAX),
        }
    }

    /// Attach an identifier used by the control-plane registry for remote reset/introspection.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn validate(&self) -> Result<(), CircuitBreakerError> {
        if *self.failure_threshold.get() == 0 {
            return Err(CircuitBreakerError::InvalidFailureThreshold { provided: 0 });
        }
        if self.recovery_timeout.get().is_zero() {
            return Err(CircuitBreakerError::InvalidRecoveryTimeout(*self.recovery_timeout.get()));
        }
        if *self.half_open_max_calls.get() == 0 {
            return Err(CircuitBreakerError::InvalidHalfOpenLimit { provided: 0 });
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct CircuitBreakerState {
    state: AtomicU8,
    failure_count: AtomicUsize,
    opened_at_millis: AtomicU64,
    half_open_calls: AtomicUsize,
}

impl CircuitBreakerState {
    pub(crate) fn new() -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed.to_u8()),
            failure_count: AtomicUsize::new(0),
            opened_at_millis: AtomicU64::new(0),
            half_open_calls: AtomicUsize::new(0),
        }
    }

    pub(crate) fn reset(&self) {
        self.state.store(CircuitState::Closed.to_u8(), Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
        self.opened_at_millis.store(0, Ordering::SeqCst);
        self.half_open_calls.store(0, Ordering::SeqCst);
    }

    pub(crate) fn current_state(&self) -> CircuitState {
        CircuitState::from_u8(self.state.load(Ordering::SeqCst))
    }
}

use crate::telemetry::{
    emit_best_effort, CircuitBreakerEvent, NullSink, PolicyEvent, RequestOutcome,
};
use std::time::Instant as StdInstant;

/// Tower-native circuit breaker layer with optional telemetry.
#[derive(Debug, Clone)]
pub struct CircuitBreakerLayer<Sink = NullSink> {
    config: CircuitBreakerConfig,
    clock: Arc<dyn Clock>,
    sink: Sink,
}

impl CircuitBreakerLayer<NullSink> {
    /// Create a new circuit breaker layer with the given configuration and no telemetry.
    ///
    /// Uses the default monotonic clock for timing.
    ///
    /// # Errors
    ///
    /// Returns error if the configuration is invalid.
    pub fn new(config: CircuitBreakerConfig) -> Result<Self, CircuitBreakerError> {
        config.validate()?;
        Ok(Self { config, clock: Arc::new(MonotonicClock::default()), sink: NullSink })
    }

    /// Create a circuit breaker layer with a custom clock implementation and no telemetry.
    ///
    /// Useful for testing with controllable time.
    ///
    /// # Errors
    ///
    /// Returns error if the configuration is invalid.
    pub fn with_clock<C: Clock + 'static>(
        config: CircuitBreakerConfig,
        clock: C,
    ) -> Result<Self, CircuitBreakerError> {
        config.validate()?;
        Ok(Self { config, clock: Arc::new(clock), sink: NullSink })
    }
}

impl<Sink> CircuitBreakerLayer<Sink>
where
    Sink: Clone,
{
    /// Attach a telemetry sink to this circuit breaker layer.
    pub fn with_sink<NewSink>(self, sink: NewSink) -> CircuitBreakerLayer<NewSink>
    where
        NewSink: Clone,
    {
        CircuitBreakerLayer { config: self.config, clock: self.clock, sink }
    }
}

/// Service produced by [`CircuitBreakerLayer`].
#[derive(Debug, Clone)]
pub struct CircuitBreakerService<S, Sink = NullSink> {
    inner: S,
    state: Arc<CircuitBreakerState>,
    config: CircuitBreakerConfig,
    clock: Arc<dyn Clock>,
    sink: Sink,
}

impl<S, Sink> CircuitBreakerService<S, Sink> {
    fn new(inner: S, config: CircuitBreakerConfig, clock: Arc<dyn Clock>, sink: Sink) -> Self {
        let state = Arc::new(CircuitBreakerState::new());

        if let Some(id) = config.id() {
            crate::circuit_breaker_registry::register_global(id.to_string(), state.clone());
        }

        Self { inner, state, config, clock, sink }
    }
}

impl<S, Request, Sink> Service<Request> for CircuitBreakerService<S, Sink>
where
    S: Service<Request> + Clone + Send + 'static,
    Request: Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    Sink: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    Sink::Error: std::error::Error + Send + 'static,
    Sink::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = ResilienceError<S::Error>;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(ResilienceError::Inner)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut inner = self.inner.clone();
        let state = self.state.clone();
        let config = self.config.clone();
        let clock = self.clock.clone();
        let sink = self.sink.clone();

        Box::pin(async move {
            let start = StdInstant::now();
            let now = clock.now_millis();
            let current = CircuitState::from_u8(state.state.load(Ordering::Acquire));
            let recovery_timeout = *config.recovery_timeout.get();
            let failure_threshold = *config.failure_threshold.get();
            let half_open_max_calls = *config.half_open_max_calls.get();

            match current {
                CircuitState::Open => {
                    let opened_at = state.opened_at_millis.load(Ordering::Acquire);
                    if now.saturating_sub(opened_at) < recovery_timeout.as_millis() as u64 {
                        return Err(ResilienceError::CircuitOpen {
                            failure_count: state.failure_count.load(Ordering::Acquire),
                            open_duration: Duration::from_millis(now.saturating_sub(opened_at)),
                        });
                    }
                    // Transition Open -> HalfOpen
                    let prev = state.state.compare_exchange(
                        CircuitState::Open.to_u8(),
                        CircuitState::HalfOpen.to_u8(),
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    );
                    if prev.is_ok() {
                        emit_best_effort(
                            sink.clone(),
                            PolicyEvent::CircuitBreaker(CircuitBreakerEvent::HalfOpen),
                        )
                        .await;
                    }
                    state.half_open_calls.store(0, Ordering::Release);
                }
                CircuitState::HalfOpen => {
                    let calls = state.half_open_calls.fetch_add(1, Ordering::AcqRel) + 1;
                    if calls > half_open_max_calls {
                        return Err(ResilienceError::CircuitOpen {
                            failure_count: state.failure_count.load(Ordering::Acquire),
                            open_duration: Duration::ZERO,
                        });
                    }
                }
                CircuitState::Closed => {}
            }

            match inner.call(req).await {
                Ok(resp) => {
                    let prev_state = CircuitState::from_u8(state.state.load(Ordering::Acquire));
                    state.state.store(CircuitState::Closed.to_u8(), Ordering::Release);
                    state.failure_count.store(0, Ordering::Release);

                    // Emit closed event if transitioning from non-closed state
                    if prev_state != CircuitState::Closed {
                        emit_best_effort(
                            sink.clone(),
                            PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Closed),
                        )
                        .await;
                    }

                    // Emit success outcome
                    let duration = start.elapsed();
                    emit_best_effort(
                        sink.clone(),
                        PolicyEvent::Request(RequestOutcome::Success { duration }),
                    )
                    .await;

                    Ok(resp)
                }
                Err(err) => {
                    let failures = state.failure_count.fetch_add(1, Ordering::AcqRel) + 1;
                    match CircuitState::from_u8(state.state.load(Ordering::Acquire)) {
                        CircuitState::Closed => {
                            if failures >= failure_threshold {
                                // Transition Closed -> Open
                                let prev = state.state.compare_exchange(
                                    CircuitState::Closed.to_u8(),
                                    CircuitState::Open.to_u8(),
                                    Ordering::AcqRel,
                                    Ordering::Acquire,
                                );
                                if prev.is_ok() {
                                    emit_best_effort(
                                        sink.clone(),
                                        PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Opened {
                                            failure_count: failures,
                                        }),
                                    )
                                    .await;
                                }
                                state.half_open_calls.store(0, Ordering::Release);
                                state.opened_at_millis.store(clock.now_millis(), Ordering::Release);
                            }
                        }
                        CircuitState::HalfOpen => {
                            let _ = state.state.compare_exchange(
                                CircuitState::HalfOpen.to_u8(),
                                CircuitState::Open.to_u8(),
                                Ordering::AcqRel,
                                Ordering::Acquire,
                            );
                            state.half_open_calls.store(0, Ordering::Release);
                            state.opened_at_millis.store(clock.now_millis(), Ordering::Release);
                        }
                        CircuitState::Open => {}
                    }
                    Err(ResilienceError::Inner(err))
                }
            }
        })
    }
}

impl<S, Sink> Layer<S> for CircuitBreakerLayer<Sink>
where
    Sink: Clone,
{
    type Service = CircuitBreakerService<S, Sink>;
    fn layer(&self, service: S) -> Self::Service {
        CircuitBreakerService::new(
            service,
            self.config.clone(),
            self.clock.clone(),
            self.sink.clone(),
        )
    }
}
