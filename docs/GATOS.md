# Nine Lives ❤️ GATOS Integration Guide

> **Note**: Nine Lives is a general-purpose resilience library for async Rust. This document describes how it integrates with [GATOS](https://github.com/flyingrobots/gatos), the primary system it was designed to support. Nine Lives is fully usable independently of GATOS.

---

## What is GATOS?

**GATOS** (Git As The Operating Surface) is a revolutionary distributed system that transforms Git from passive version control into an active, self-governing computational substrate. It provides five integrated planes:

1. **Ledger Plane** - Append-only event logs as the source of truth
2. **State Plane** - Deterministic checkpoints derived via pure folds
3. **Policy/Trust Plane** - Executable governance rules with cryptographic grants
4. **Message Plane** - Commit-backed pub/sub for asynchronous messaging
5. **Job Plane** - Distributed, verifiable job execution with Proofs-of-Execution

GATOS enables **verifiable, auditable infrastructure** where all state, policy, and computation history lives in Git as immutable commits.

**Repository**: <https://github.com/flyingrobots/gatos>

---

## Why GATOS Needs Nine Lives

GATOS provides **deterministic correctness** and **cryptographic verifiability**. Nine Lives provides the **operational resilience** needed to make GATOS production-ready:

| GATOS Requirement | Challenge | Nine Lives Solution |
|-------------------|-----------|---------------------|
| **At-least-once job delivery** | Workers can crash mid-execution | Retry policies with exponential backoff |
| **Policy gate under load** | Expensive policy evaluation can saturate | Bulkhead to limit concurrent evaluations |
| **Opaque pointer resolution** | KMS/blob storage can fail transiently | Circuit breaker + retry + timeout |
| **Message bus publishing** | Git CAS can race under high concurrency | Adaptive backoff + jitter |
| **Audit trail integrity** | Audit writes must never fail silently | Guaranteed audit sink + fallback composition |
| **Worker pool scaling** | Need to auto-tune concurrency | Adaptive bulkhead patterns |
| **Policy decision diversity** | Want fast path with fallback | Fork-join (`&`) operator for Happy Eyeballs |

---

## Nine Lives Feature → GATOS Use Case Mapping

### Core Resilience Patterns

#### 1. **Retry Policies** (`RetryLayer`)

**Features:**

- Configurable backoff strategies (constant, linear, exponential)
- Jitter modes (full, equal, decorrelated) to prevent thundering herds
- Adaptive max attempts via `Adaptive<usize>` handles
- Custom retry predicates (which errors are retryable?)

**GATOS Use Cases:**

```rust
// Job execution - retry transient failures
use ninelives::prelude::*;

let job_executor = ServiceBuilder::new()
    .layer(RetryPolicy::builder()
        .max_attempts(3)
        .backoff(Backoff::exponential(Duration::from_millis(100)))
        .with_jitter(Jitter::decorrelated())  // AWS-style jitter
        .should_retry(|e| e.is_transient())   // Only retry transient errors
        .build()?)
    .service(job_worker);

// Message publishing - handle Git CAS races
let publisher = ServiceBuilder::new()
    .layer(RetryPolicy::builder()
        .max_attempts(10)  // Git CAS can race frequently under load
        .backoff(Backoff::exponential(Duration::from_millis(50)))
        .with_jitter(Jitter::full())  // Randomize to reduce collisions
        .build()?)
    .service(git_message_publisher);

// Opaque pointer fetch - retry network failures
let blob_fetcher = ServiceBuilder::new()
    .layer(RetryPolicy::builder()
        .max_attempts(5)
        .backoff(Backoff::linear(Duration::from_millis(200)))
        .should_retry(|e| matches!(e, BlobError::NetworkTimeout | BlobError::ServiceUnavailable))
        .build()?)
    .service(s3_client);
```

#### 2. **Circuit Breaker** (`CircuitBreakerLayer`)

**Features:**

- Three-state machine: Closed → Open → HalfOpen
- Configurable failure threshold, recovery timeout, half-open test calls
- Adaptive thresholds via `Adaptive<T>` handles
- Lock-free implementation using atomics

**GATOS Use Cases:**

```rust
// Protect KMS from cascading failures
let kms_client = ServiceBuilder::new()
    .layer(CircuitBreakerLayer::new(
        CircuitBreakerConfig::new(
            5,                              // Open after 5 consecutive failures
            Duration::from_secs(30),        // Stay open for 30s
            3,                              // Allow 3 test calls in half-open
        )?
    )?)
    .service(actual_kms_client);

// Policy evaluation - prevent overload
let policy_gate = ServiceBuilder::new()
    .layer(CircuitBreakerLayer::new(
        CircuitBreakerConfig::new(
            10,                             // Open after 10 failures
            Duration::from_secs(10),        // Quick recovery attempt
            5,                              // More test calls (policy is critical)
        )?
    )?)
    .service(policy_vm);

// External Git remote - handle network partitions
let git_remote = ServiceBuilder::new()
    .layer(CircuitBreakerLayer::new(
        CircuitBreakerConfig::new(
            3,                              // Fast trip on network issues
            Duration::from_secs(60),        // Wait longer for network recovery
            2,
        )?
    )?)
    .service(git_push_service);
```

#### 3. **Bulkhead** (`BulkheadLayer`)

**Features:**

- Concurrency limiting via semaphore
- Adaptive concurrency via `Adaptive<usize>` handles
- Immediate rejection when saturated (no queueing)

**GATOS Use Cases:**

```rust
// Limit concurrent policy evaluations
let policy_gate = ServiceBuilder::new()
    .layer(BulkheadLayer::new(100)?)  // Max 100 concurrent evals
    .service(policy_vm);

// Limit concurrent job executions per worker
let job_worker = ServiceBuilder::new()
    .layer(BulkheadLayer::new(10)?)   // Max 10 concurrent jobs
    .service(job_executor);

// Limit concurrent blob fetches
let blob_resolver = ServiceBuilder::new()
    .layer(BulkheadLayer::new(50)?)   // Max 50 concurrent S3 requests
    .service(s3_client);

// Adaptive bulkhead - auto-tune based on load
let adaptive_bulkhead = BulkheadLayer::new(100)?;
let handle = adaptive_bulkhead.adaptive_max_concurrent();

// Later, tune based on metrics:
if success_rate < 0.9 {
    handle.set(50);  // Reduce concurrency under stress
} else if success_rate > 0.99 {
    handle.set(150); // Increase concurrency when healthy
}
```

#### 4. **Timeout** (`TimeoutLayer`)

**Features:**

- Tokio-integrated timeouts
- Adaptive duration via `Adaptive<Duration>` handles
- Per-request timeout enforcement

**GATOS Use Cases:**

```rust
// Bounded job execution time
let job_worker = ServiceBuilder::new()
    .layer(TimeoutLayer::new(Duration::from_secs(300))?)  // 5 min max
    .service(job_executor);

// Fast policy evaluation timeout
let policy_gate = ServiceBuilder::new()
    .layer(TimeoutLayer::new(Duration::from_millis(500))?)
    .service(policy_vm);

// Network operation timeout
let blob_fetcher = ServiceBuilder::new()
    .layer(TimeoutLayer::new(Duration::from_secs(10))?)
    .service(s3_client);
```

### Algebraic Composition

#### 5. **Sequential Composition** (`Policy(A) + Policy(B)`)

**Features:**

- Stack layers: `A` wraps `B`
- Standard middleware pattern
- Precedence: `+` binds tighter than `|`

**GATOS Use Cases:**

```rust
// Chain timeout → circuit breaker → retry → bulkhead
let robust_service = Policy(TimeoutLayer::new(Duration::from_secs(30))?)
    + Policy(CircuitBreakerLayer::new(config)?)
    + Policy(RetryLayer::new(...)?)
    + Policy(BulkheadLayer::new(50)?);

// Apply to job worker
let worker = ServiceBuilder::new()
    .layer(robust_service)
    .service(job_executor);
```

#### 6. **Fallback Composition** (`Policy(A) | Policy(B)`)

**Features:**

- Try primary strategy, fall back to secondary on error
- Original request retried with fallback stack
- Use for graceful degradation

**GATOS Use Cases:**

```rust
// Fast policy evaluation with slow fallback
let fast = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
let slow = Policy(TimeoutLayer::new(Duration::from_secs(5))?)
         + Policy(RetryLayer::new(...)?)
         + Policy(BulkheadLayer::new(10)?);

let policy_gate = ServiceBuilder::new()
    .layer(fast | slow)  // Try fast, fall back to slow on timeout
    .service(policy_vm);

// Primary blob storage with fallback
let primary_storage = Policy(TimeoutLayer::new(Duration::from_secs(2))?);
let backup_storage = Policy(TimeoutLayer::new(Duration::from_secs(10))?);

let blob_fetcher = ServiceBuilder::new()
    .layer(primary_storage | backup_storage)
    .service(storage_client);
```

#### 7. **Fork-Join / Happy Eyeballs** (`Policy(A) & Policy(B)`)

**Features:**

- Race both strategies concurrently
- Return first successful result
- Use for low-latency diversity

**GATOS Use Cases:**

```rust
// Race two policy evaluation strategies
let fast_approx = Policy(TimeoutLayer::new(Duration::from_millis(50))?);
let slow_exact = Policy(TimeoutLayer::new(Duration::from_secs(2))?);

let policy_gate = ServiceBuilder::new()
    .layer(fast_approx & slow_exact)  // First success wins
    .service(policy_vm);

// Race local cache vs. remote fetch
let from_cache = Policy(TimeoutLayer::new(Duration::from_millis(10))?);
let from_remote = Policy(TimeoutLayer::new(Duration::from_secs(1))?);

let blob_fetcher = ServiceBuilder::new()
    .layer(from_cache & from_remote)  // Cache miss? Remote fetch racing
    .service(blob_client);

// IPv4 vs IPv6 Happy Eyeballs for Git push
let ipv4_path = Policy(TimeoutLayer::new(Duration::from_millis(300))?);
let ipv6_path = Policy(TimeoutLayer::new(Duration::from_millis(300))?);

let git_push = ServiceBuilder::new()
    .layer(ipv4_path & ipv6_path)
    .service(git_client);
```

### Telemetry & Observability

#### 8. **Telemetry Sinks** (`TelemetrySink`)

**Features:**

- Rich `PolicyEvent` enum (retry attempts, circuit opened, bulkhead rejected, etc.)
- Built-in sinks: `NullSink`, `LogSink`, `MemorySink`, `StreamingSink`
- Composable sinks: `MulticastSink`, `FallbackSink`
- Non-blocking wrapper: `NonBlockingSink`

**GATOS Use Cases:**

```rust
// GATOS-specific audit sink (extend Nine Lives)
use ninelives::telemetry::{PolicyEvent, TelemetrySink};

#[derive(Clone)]
struct GatosAuditSink {
    ledger: Arc<GatosLedger>,
}

impl Service<PolicyEvent> for GatosAuditSink {
    type Response = ();
    type Error = AuditError;
    type Future = BoxFuture<'static, Result<(), AuditError>>;

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let ledger = self.ledger.clone();
        Box::pin(async move {
            // Write event to refs/gatos/audit/ninelives/*
            ledger.append_audit_event(event).await
        })
    }
}

// Compose with built-in sinks
let telemetry = MulticastSink::new(
    LogSink,                        // Log to tracing
    MulticastSink::new(
        MemorySink::new(),          // In-memory buffer
        GatosAuditSink::new(ledger) // Write to GATOS audit refs
    )
);

// Attach to retry policy
let retry = RetryLayer::new(...)?.with_sink(telemetry);

// Or use streaming sink for fan-out
let stream = StreamingSink::new(1000);
let mut subscriber = stream.subscribe();

tokio::spawn(async move {
    while let Ok(event) = subscriber.recv().await {
        // Process events (e.g., update metrics, trigger alerts)
        process_event(event).await;
    }
});

let retry = RetryLayer::new(...)?.with_sink(stream);
```

### Control Plane & Runtime Tuning

#### 9. **Adaptive Configuration** (`Adaptive<T>`)

**Features:**

- Lock-free reads via `arc-swap` (default) or `RwLock` (feature flag)
- Live updates without redeployment
- Integrates with control plane commands

**GATOS Use Cases:**

```rust
// Create adaptive retry policy
let retry = RetryPolicy::builder()
    .max_attempts(3)
    .backoff(Backoff::exponential(Duration::from_millis(100)))
    .build()?;

let adaptive_retry = retry.adaptive_max_attempts();

// Expose via GATOS command interface
gatos_control.register("ninelives.retry.max_attempts", adaptive_retry);

// Later, tune via command:
// $ gatos cmd write ninelives.retry.max_attempts 5
adaptive_retry.set(5);  // Now retries up to 5 times
```

#### 10. **Control Plane** (`CommandRouter`, `ConfigRegistry`)

**Features:**

- Transport-agnostic command infrastructure
- Pluggable auth/audit
- Built-in handlers: Set/Get/List/Reset, ReadConfig/WriteConfig
- Command history tracking

**GATOS Use Cases:**

```rust
use ninelives::control::*;

// Register adaptive policies with control plane
let mut config_registry = ConfigRegistry::new();
config_registry.register_fromstr("retry.max_attempts", retry.adaptive_max_attempts());
config_registry.register_fromstr("bulkhead.max_concurrent", bulkhead.adaptive_max_concurrent());
config_registry.register_fromstr("circuit.failure_threshold", breaker.adaptive_threshold());

let handler = BuiltInHandler::default().with_config_registry(config_registry);

// Set up auth (integrate with GATOS trust graph)
let mut auth = AuthRegistry::new(AuthMode::First);
auth.register(Arc::new(GatosTrustAuth::new(trust_graph)));

// Create router with audit sink
let router = CommandRouter::new(
    auth,
    Arc::new(handler),
    Arc::new(GatosCommandHistory::new(ledger.clone()))
).with_audit(Arc::new(GatosAuditSink::new(ledger.clone())));

// Execute commands
let cmd = CommandEnvelope {
    cmd: BuiltInCommand::WriteConfig {
        path: "retry.max_attempts".into(),
        value: "5".into(),
    },
    auth: Some(AuthPayload::from_gatos_grant(grant)),
    meta: CommandMeta { id: ulid(), correlation_id: None, timestamp_millis: None },
};

router.execute(cmd).await?;
```

---

## Complete Integration Plan

### Architecture Overview

```text
┌─────────────────────────────────────────────────────────┐
│                    GATOS Application                     │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐        │
│  │ Job Worker │  │ Policy Gate│  │  Msg Bus   │        │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘        │
│        │               │               │                │
│        └───────────────┴───────────────┘                │
│                        │                                │
│              ┌─────────▼──────────┐                     │
│              │   Nine Lives       │                     │
│              │  Resilience Layers │                     │
│              └─────────┬──────────┘                     │
│                        │                                │
│        ┌───────────────┼───────────────┐               │
│        │               │               │               │
│   ┌────▼─────┐   ┌────▼─────┐   ┌────▼─────┐         │
│   │ Retry    │   │ Circuit  │   │ Bulkhead │         │
│   │ Timeout  │   │ Breaker  │   │          │         │
│   └────┬─────┘   └────┬─────┘   └────┬─────┘         │
│        │              │              │                │
│        └──────────────┴──────────────┘                │
│                       │                               │
│              ┌────────▼─────────┐                     │
│              │  Tower Services  │                     │
│              │  (Git, KMS, etc) │                     │
│              └──────────────────┘                     │
│                                                        │
│        Telemetry Events ──────────┐                   │
│                                    │                   │
│                        ┌───────────▼────────────┐     │
│                        │  GATOS Audit Refs      │     │
│                        │  refs/gatos/audit/     │     │
│                        └────────────────────────┘     │
└─────────────────────────────────────────────────────────┘
```

### Integration Steps by GATOS Component

#### 1. **Job Plane (gatos-compute)**

**Objective**: Ensure job workers execute reliably despite transient failures and generate Proofs-of-Execution.

**Implementation:**

```rust
// In gatos-compute/src/worker.rs

use ninelives::prelude::*;
use tower::ServiceBuilder;

pub struct JobWorker {
    executor: tower::util::BoxService<Job, ProofOfExecution, JobError>,
}

impl JobWorker {
    pub fn new(raw_executor: impl JobExecutor) -> Result<Self, WorkerError> {
        // Define resilience strategy
        let policy = Policy(TimeoutLayer::new(Duration::from_secs(300))?)  // 5min hard timeout
            + Policy(CircuitBreakerLayer::new(
                CircuitBreakerConfig::new(5, Duration::from_secs(60), 3)?
            )?)
            + Policy(RetryPolicy::builder()
                .max_attempts(3)
                .backoff(Backoff::exponential(Duration::from_millis(100)))
                .with_jitter(Jitter::decorrelated())
                .should_retry(|e| e.is_retryable())
                .build()?)
            + Policy(BulkheadLayer::new(10)?);  // Max 10 concurrent jobs

        let executor = ServiceBuilder::new()
            .layer(policy)
            .service(raw_executor);

        Ok(Self { executor: tower::util::BoxService::new(executor) })
    }

    pub async fn execute(&mut self, job: Job) -> Result<ProofOfExecution, JobError> {
        self.executor.call(job).await
    }
}
```

**Telemetry Integration:**

```rust
// Create GATOS-specific audit sink
let audit_sink = GatosAuditSink::new(ledger.clone());

// Attach to retry layer
let retry = RetryPolicy::builder()
    .max_attempts(3)
    .build()?
    .into_layer()
    .with_sink(audit_sink);

// All retry events now written to refs/gatos/audit/ninelives/
```

#### 2. **Policy Gate (gatos-policy)**

**Objective**: Protect policy evaluation from overload while ensuring governance decisions complete.

**Implementation:**

```rust
// In gatos-policy/src/gate.rs

use ninelives::prelude::*;

pub struct PolicyGate {
    evaluator: tower::util::BoxService<Intent, Decision, PolicyError>,
    config: Adaptive<GateConfig>,
}

impl PolicyGate {
    pub fn new(vm: PolicyVM, ledger: Arc<GatosLedger>) -> Result<Self, GateError> {
        let audit_sink = GatosAuditSink::new(ledger);

        // Fast path with fallback to comprehensive evaluation
        let fast = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
        let slow = Policy(TimeoutLayer::new(Duration::from_secs(5))?)
                 + Policy(BulkheadLayer::new(100)?);

        let policy = (fast | slow)
            + Policy(RetryPolicy::builder()
                .max_attempts(1)  // Policy eval should be deterministic
                .build()?)
            .with_sink(audit_sink);

        let evaluator = ServiceBuilder::new()
            .layer(policy)
            .service(vm);

        Ok(Self {
            evaluator: tower::util::BoxService::new(evaluator),
            config: Adaptive::new(GateConfig::default()),
        })
    }

    pub async fn evaluate(&mut self, intent: Intent) -> Result<Decision, PolicyError> {
        self.evaluator.call(intent).await
    }
}
```

#### 3. **Message Plane (gatos-message-plane)**

**Objective**: Handle backpressure and Git CAS races during high-throughput publishing.

**Implementation:**

```rust
// In gatos-message-plane/src/publisher.rs

use ninelives::prelude::*;

pub struct MessagePublisher {
    git_writer: tower::util::BoxService<Message, CommitId, PublishError>,
}

impl MessagePublisher {
    pub fn new(git_client: GitClient) -> Result<Self, PublisherError> {
        // Git CAS can race frequently - use high retry count with jitter
        let policy = Policy(RetryPolicy::builder()
            .max_attempts(10)  // Git CAS races require persistence
            .backoff(Backoff::exponential(Duration::from_millis(50)))
            .with_jitter(Jitter::full())  // Randomize to reduce collision
            .should_retry(|e| matches!(e, PublishError::CasRace))
            .build()?)
            + Policy(TimeoutLayer::new(Duration::from_secs(5))?);

        let git_writer = ServiceBuilder::new()
            .layer(policy)
            .service(git_client);

        Ok(Self { git_writer: tower::util::BoxService::new(git_writer) })
    }

    pub async fn publish(&mut self, msg: Message) -> Result<CommitId, PublishError> {
        self.git_writer.call(msg).await
    }
}
```

#### 4. **Opaque Pointer Resolution (gatos-privacy)**

**Objective**: Resilient blob fetching from external stores (S3, KMS) with circuit breaking.

**Implementation:**

```rust
// In gatos-privacy/src/resolver.rs

use ninelives::prelude::*;

pub struct OpaquePointerResolver {
    blob_fetcher: tower::util::BoxService<OpaquePointer, Vec<u8>, ResolveError>,
    kms_client: tower::util::BoxService<KeyRequest, Key, KmsError>,
}

impl OpaquePointerResolver {
    pub fn new(s3: S3Client, kms: KmsClient) -> Result<Self, ResolverError> {
        // Blob fetching: retry transient failures, circuit break on persistent issues
        let blob_policy = Policy(CircuitBreakerLayer::new(
            CircuitBreakerConfig::new(5, Duration::from_secs(30), 3)?
        )?)
            + Policy(RetryPolicy::builder()
                .max_attempts(5)
                .backoff(Backoff::exponential(Duration::from_millis(200)))
                .should_retry(|e| e.is_transient())
                .build()?)
            + Policy(TimeoutLayer::new(Duration::from_secs(10))?);

        let blob_fetcher = ServiceBuilder::new()
            .layer(blob_policy)
            .service(s3);

        // KMS: circuit break aggressively (KMS outage is serious)
        let kms_policy = Policy(CircuitBreakerLayer::new(
            CircuitBreakerConfig::new(3, Duration::from_secs(60), 2)?
        )?)
            + Policy(RetryPolicy::builder()
                .max_attempts(3)
                .backoff(Backoff::linear(Duration::from_secs(1)))
                .build()?)
            + Policy(TimeoutLayer::new(Duration::from_secs(5))?);

        let kms_client = ServiceBuilder::new()
            .layer(kms_policy)
            .service(kms);

        Ok(Self {
            blob_fetcher: tower::util::BoxService::new(blob_fetcher),
            kms_client: tower::util::BoxService::new(kms_client),
        })
    }
}
```

#### 5. **Control Plane Integration**

**Objective**: Expose Nine Lives adaptive handles via GATOS command interface.

**Implementation:**

```rust
// In gatos-control/src/ninelives_integration.rs

use ninelives::control::*;

pub struct NineLivesControl {
    router: CommandRouter<BuiltInCommand>,
    config: ConfigRegistry,
}

impl NineLivesControl {
    pub fn new(ledger: Arc<GatosLedger>, trust: Arc<TrustGraph>) -> Self {
        let mut config = ConfigRegistry::new();

        // Register all adaptive handles
        config.register_fromstr("job.retry.max_attempts", job_retry.adaptive_max_attempts());
        config.register_fromstr("job.bulkhead.max_concurrent", job_bulkhead.adaptive_max_concurrent());
        config.register_fromstr("job.timeout.duration", job_timeout.adaptive_duration());
        config.register_fromstr("policy.bulkhead.max_concurrent", policy_bulkhead.adaptive_max_concurrent());
        config.register_fromstr("policy.timeout.duration", policy_timeout.adaptive_duration());
        config.register_fromstr("message.retry.max_attempts", msg_retry.adaptive_max_attempts());
        config.register_fromstr("blob.circuit.failure_threshold", blob_circuit.adaptive_threshold());

        let handler = BuiltInHandler::default().with_config_registry(config);

        // Integrate with GATOS auth
        let mut auth = AuthRegistry::new(AuthMode::First);
        auth.register(Arc::new(GatosTrustAuth::new(trust)));

        let router = CommandRouter::new(
            auth,
            Arc::new(handler),
            Arc::new(GatosCommandHistory::new(ledger.clone()))
        ).with_audit(Arc::new(GatosAuditSink::new(ledger)));

        Self { router, config }
    }

    pub async fn handle_command(&self, cmd: GatosCommand) -> Result<CommandResult, CommandError> {
        // Convert GATOS command to Nine Lives CommandEnvelope
        let envelope = CommandEnvelope {
            cmd: cmd.into_builtin(),
            auth: Some(AuthPayload::from_gatos_grant(cmd.grant)),
            meta: CommandMeta {
                id: cmd.id.to_string(),
                correlation_id: cmd.correlation_id.map(|u| u.to_string()),
                timestamp_millis: Some(cmd.timestamp_millis),
            },
        };

        self.router.execute(envelope).await
    }
}
```

### Custom GATOS Extensions

#### GATOS Audit Sink

```rust
// gatos-ninelives/src/audit_sink.rs

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use tower::Service;
use gatos_ledger_git::GatosLedger;

#[derive(Clone)]
pub struct GatosAuditSink {
    ledger: Arc<GatosLedger>,
}

impl GatosAuditSink {
    pub fn new(ledger: Arc<GatosLedger>) -> Self {
        Self { ledger }
    }
}

impl Service<PolicyEvent> for GatosAuditSink {
    type Response = ();
    type Error = AuditError;
    type Future = BoxFuture<'static, Result<(), AuditError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let ledger = self.ledger.clone();
        Box::pin(async move {
            // Serialize event to canonical JSON
            let envelope = AuditEnvelope {
                timestamp: SystemTime::now(),
                event_type: event.to_string(),
                payload: serde_json::to_value(&event)?,
            };

            let json = serde_json::to_string(&envelope)?;

            // Write to refs/gatos/audit/ninelives/<timestamp>
            ledger.append_audit_ref(
                "ninelives",
                json.as_bytes(),
                &envelope.timestamp,
            ).await?;

            Ok(())
        })
    }
}

impl TelemetrySink for GatosAuditSink {
    type SinkError = AuditError;
}
```

#### GATOS Trust Auth Provider

```rust
// gatos-ninelives/src/trust_auth.rs

use ninelives::control::{AuthProvider, AuthContext, AuthError, CommandMeta, AuthPayload};
use gatos_policy::TrustGraph;

pub struct GatosTrustAuth {
    trust: Arc<TrustGraph>,
}

impl GatosTrustAuth {
    pub fn new(trust: Arc<TrustGraph>) -> Self {
        Self { trust }
    }
}

impl AuthProvider for GatosTrustAuth {
    fn name(&self) -> &'static str {
        "gatos-trust"
    }

    fn authenticate(
        &self,
        meta: &CommandMeta,
        auth: Option<&AuthPayload>,
    ) -> Result<AuthContext, AuthError> {
        let grant = auth
            .ok_or_else(|| AuthError::Unauthenticated("no grant provided".into()))?;

        // Extract GATOS grant from auth payload
        let gatos_grant = match grant {
            AuthPayload::Signatures { payload_hash, signatures } => {
                self.trust.verify_grant(payload_hash, signatures)
                    .map_err(|e| AuthError::Unauthenticated(e.to_string()))?
            }
            _ => return Err(AuthError::Unauthenticated("unsupported auth type".into())),
        };

        Ok(AuthContext {
            principal: gatos_grant.actor_id.to_string(),
            provider: self.name(),
            attributes: gatos_grant.capabilities.into_iter()
                .map(|cap| (cap.resource.clone(), cap.action.clone()))
                .collect(),
        })
    }

    fn authorize(
        &self,
        ctx: &AuthContext,
        label: &str,
        _meta: &CommandMeta,
    ) -> Result<(), AuthError> {
        // Check if actor has capability to execute this command
        let required_cap = format!("ninelives.{}", label);

        if ctx.attributes.contains_key(&required_cap) {
            Ok(())
        } else {
            Err(AuthError::Unauthorized(format!(
                "actor {} lacks capability {}",
                ctx.principal, required_cap
            )))
        }
    }
}
```

---

## Mapping: Nine Lives Phases → GATOS Milestones

### Nine Lives P1: Observability Foundation ✅

**Status**: Complete
**Features**: Telemetry events, sinks, composition

**GATOS Integration**: Ready for M3-M4

| NL Component | GATOS Milestone | Purpose |
|--------------|-----------------|---------|
| PolicyEvent enum | M3 (Message Bus) | Audit retry/timeout events during publishing |
| TelemetrySink trait | M2 (Policy Gate) | Log policy evaluation metrics |
| LogSink, MemorySink | M1-M2 (Development) | Debug fold/policy performance |
| StreamingSink | M6 (Explorer) | Real-time metrics for dashboard |

---

### Nine Lives P2: Runtime Adaptive Tuning 🚧

**Status**: ~50% complete (P2.01-P2.10, P2.17 closed)
**Features**: Control plane, adaptive handles, config commands

**GATOS Integration**: Enables M4-M5 operational tuning

| NL Component | GATOS Milestone | Purpose |
|--------------|-----------------|---------|
| Adaptive<T> | M4 (Job Plane) | Tune worker concurrency live |
| CommandRouter | M4 (Job Plane) | Control job execution parameters |
| ConfigRegistry | M2 (Policy Gate) | Adjust policy eval timeout/bulkhead |
| WriteConfig/ReadConfig | M5 (Privacy) | Tune blob fetch retry/circuit breaker |
| AuthRegistry | M2 (Governance) | Integrate with GATOS trust graph |
| AuditSink | M2 (Governance) | Record all config changes to audit refs |

**Critical for GATOS:**

- P2.14 (Transport abstraction) - needed for M8 (GATOS CLI integration)
- P2.16 (AuthZ layer) - needed for M2 (governance integration)
- P2.19 (ninelives-control crate) - separate control plane for GATOS

---

### Nine Lives P3: Adaptive Policies 📋

**Status**: Planned
**Features**: Auto-tuning retry/breaker/bulkhead based on telemetry

**GATOS Integration**: Enables M6-M7 autonomous operation

| NL Component | GATOS Milestone | Purpose |
|--------------|-----------------|---------|
| Adaptive retry | M4 (Job Plane) | Auto-tune retry attempts based on job success rate |
| Adaptive bulkhead | M4 (Job Plane) | Scale worker concurrency with load |
| Adaptive circuit breaker | M5 (Privacy) | Tune KMS circuit threshold based on error patterns |
| Telemetry aggregation | M6 (Explorer) | Feed metrics into GATOS state plane |

**Critical for GATOS:**

- P3.01-P3.03: Adaptive retry (job workers need this for M4)
- P3.07-P3.09: Adaptive bulkhead (worker pool auto-scaling for M4)

---

### Nine Lives P4: Happy Eyeballs (Fork-Join) 📋

**Status**: Planned
**Features**: Concurrent racing of strategies (`&` operator)

**GATOS Integration**: Enables M5-M7 low-latency diversity

| NL Component | GATOS Milestone | Purpose |
|--------------|-----------------|---------|
| Fork-join operator | M2 (Policy Gate) | Race fast policy eval vs. comprehensive |
| Concurrent racing | M5 (Privacy) | Race local cache vs. remote blob fetch |
| First-success semantics | M3 (Message Bus) | Try multiple Git remotes concurrently |
| IPv4/IPv6 Happy Eyeballs | M10 (Enterprise) | Network diversity for distributed clusters |

**Critical for GATOS:**

- P4.01-P4.05: Happy Eyeballs implementation (policy gate fast path for M2)
- P4.08-P4.10: Concurrent fallback (blob fetch optimization for M5)

---

### Nine Lives P5: Observer & System State 📋

**Status**: Planned
**Features**: Aggregate telemetry into queryable state

**GATOS Integration**: Feeds into M6 Explorer and M7 Proof-of-Experiment

| NL Component | GATOS Milestone | Purpose |
|--------------|-----------------|---------|
| Observer service | M6 (Explorer) | Real-time resilience metrics dashboard |
| SystemState aggregation | M7 (Proof-of-Experiment) | Verifiable performance characteristics |
| Telemetry queries | M6 (Explorer) | GraphQL/REST API for metrics |
| Historical analysis | M7 (PoX) | Reproducible performance claims |

**Critical for GATOS:**

- P5.01-P5.05: Observer implementation (needed for M6 dashboard)
- P5.08-P5.10: State queries (expose metrics to GATOS Explorer)

---

### Nine Lives P6: Shadow Evaluation 📋

**Status**: Planned
**Features**: What-if analysis for policy changes

**GATOS Integration**: Critical for M9 conformance and M10 safety

| NL Component | GATOS Milestone | Purpose |
|--------------|-----------------|---------|
| Shadow policies | M9 (Conformance) | Test policy changes without affecting prod |
| What-if evaluation | M2 (Governance) | Validate new policy rules before promotion |
| Shadow metrics | M10 (Security) | Assess impact of security hardening |
| Promotion triggers | M9 (Conformance) | Auto-promote successful shadow configs |

**Critical for GATOS:**

- P6.01-P6.04: Shadow policy evaluation (governance safety for M2)
- P6.07-P6.09: Promotion logic (automated policy updates for M10)

---

### Nine Lives P7: Crate Split 📋

**Status**: Planned
**Features**: Separate ninelives-core, ninelives-control, etc.

**GATOS Integration**: Clean dependency management for GATOS crates

| NL Component | GATOS Milestone | Purpose |
|--------------|-----------------|---------|
| ninelives-core | M3-M5 (All) | Core resilience patterns (minimal deps) |
| ninelives-control | M8 (Demos) | Control plane (separate from core) |
| ninelives-observer | M6 (Explorer) | Observability (optional dependency) |
| ninelives-sentinel | M10+ (Autonomy) | Self-healing (enterprise feature) |

**Critical for GATOS:**

- P7.01-P7.05: Core/control split (allows GATOS to use patterns without control overhead)
- P7.08-P7.10: Observer separation (GATOS can substitute its own state aggregation)

---

### Nine Lives P8: Transport Adapters 📋

**Status**: Planned
**Features**: HTTP, gRPC, WebSocket control plane transports

**GATOS Integration**: Enables M8 CLI and M11 community tooling

| NL Component | GATOS Milestone | Purpose |
|--------------|-----------------|---------|
| HTTP REST adapter | M8 (Demos) | GATOS CLI control commands |
| gRPC adapter | M10 (Enterprise) | High-performance control plane |
| WebSocket adapter | M6 (Explorer) | Real-time dashboard updates |
| In-process channels | M4 (Job Plane) | Worker-local control |

**Critical for GATOS:**

- P8.01-P8.03: HTTP adapter (enables `gatos cmd` CLI for M8)
- P8.05-P8.07: gRPC adapter (production control plane for M10)

---

### Nine Lives P9: Distributed Patterns 📋

**Status**: Planned
**Features**: Leader election, distributed tracing, coordinated resilience

**GATOS Integration**: Enables M10+ enterprise clustering

| NL Component | GATOS Milestone | Purpose |
|--------------|-----------------|---------|
| Distributed circuit breakers | M10+ (Clustering) | Cluster-wide fault isolation |
| Coordinated rate limiting | M10+ (Clustering) | Global bulkheads across workers |
| Leader election integration | M10+ (Clustering) | Sentinel coordination |
| Distributed tracing | M10+ (Observability) | Trace resilience across services |

**Critical for GATOS:**

- P9.01-P9.04: Distributed circuit breaker (cluster-wide KMS protection)
- P9.07-P9.09: Distributed tracing (cross-service observability)

---

### Nine Lives P10: Production Hardening 📋

**Status**: Planned
**Features**: Benchmarks, zero-overhead optimizations, security audits

**GATOS Integration**: Production readiness for M10-M11

| NL Component | GATOS Milestone | Purpose |
|--------------|-----------------|---------|
| Benchmarks | M9 (Conformance) | Validate performance claims |
| Zero-allocation paths | M10 (Security) | Minimize attack surface |
| Fuzzing | M10 (Security) | Find edge cases in resilience logic |
| Security audit | M11 (Launch) | Third-party validation |

**Critical for GATOS:**

- P10.01-P10.04: Benchmarks (performance baseline for PoX)
- P10.09-P10.12: Security hardening (required for M11 launch)

---

## Milestone-Driven Roadmap Summary

### GATOS M1-M2: Foundation (Current Phase)

**Nine Lives Requirements:**

- P2 (Control Plane) - runtime tuning for policy gate
- Basic telemetry (P1) - audit policy decisions

**Integration Priority:**

1. GatosAuditSink (write events to audit refs)
2. GatosTrustAuth (integrate with trust graph)
3. Policy gate resilience (timeout + bulkhead)

---

### GATOS M3: Message Plane

**Nine Lives Requirements:**

- P2 (Retry + Jitter) - handle Git CAS races
- P4 (Fork-Join) - race multiple Git remotes

**Integration Priority:**

1. Message publisher retry (exponential backoff + jitter)
2. Multi-remote racing (Happy Eyeballs for Git push)
3. Backpressure handling (adaptive bulkhead)

---

### GATOS M4: Job Plane

**Nine Lives Requirements:**

- P2 (Full resilience stack) - retry + circuit + bulkhead + timeout
- P3 (Adaptive patterns) - auto-tune worker concurrency
- P5 (Observer) - track job success/failure rates

**Integration Priority:**

1. Job executor resilience (complete policy stack)
2. Worker pool bulkhead (adaptive concurrency)
3. Proof-of-Execution retry (guarantee PoE generation)
4. Job metrics aggregation (feed into Observer)

---

### GATOS M5: Privacy & Opaque Pointers

**Nine Lives Requirements:**

- P2 (Circuit breaker) - protect KMS from cascading failures
- P3 (Adaptive retry) - tune blob fetch based on error rate
- P4 (Fork-join) - race cache vs. remote fetch

**Integration Priority:**

1. KMS circuit breaker (aggressive tripping)
2. Blob fetch retry (handle transient S3 errors)
3. Cache racing (local vs. remote concurrent fetch)

---

### GATOS M6: Explorer & Verification

**Nine Lives Requirements:**

- P5 (Observer) - aggregate metrics for dashboard
- P8 (HTTP adapter) - expose metrics API

**Integration Priority:**

1. Real-time metrics dashboard
2. GraphQL API for resilience state
3. Historical performance queries

---

### GATOS M7: Proof-of-Experiment

**Nine Lives Requirements:**

- P5 (Observer) - verifiable performance claims
- P6 (Shadow evaluation) - test without affecting prod

**Integration Priority:**

1. Reproducible performance benchmarks
2. Shadow policy evaluation for experiments
3. Verifiable resilience characteristics

---

### GATOS M8: Demos & Examples

**Nine Lives Requirements:**

- P8 (HTTP adapter) - CLI integration
- P2 (Complete control plane) - runtime tuning demos

**Integration Priority:**

1. `gatos cmd` CLI integration
2. Example policies (job, policy, message)
3. Cookbook recipes for common patterns

---

### GATOS M9: Conformance Suite

**Nine Lives Requirements:**

- P6 (Shadow evaluation) - test conformance without risk
- P10 (Benchmarks) - validate performance claims

**Integration Priority:**

1. Shadow policy conformance tests
2. Performance benchmarks
3. Integration test suite

---

### GATOS M10+: Enterprise & Scale

**Nine Lives Requirements:**

- P9 (Distributed patterns) - cluster-wide resilience
- P10 (Production hardening) - security audit

**Integration Priority:**

1. Distributed circuit breakers
2. Cluster-wide bulkheads
3. Security hardening
4. Third-party audit

---

## Summary

Nine Lives provides the **operational backbone** that makes GATOS **production-ready**:

- **M1-M2 (Foundation)**: Control plane integration, policy gate resilience
- **M3 (Message Bus)**: Retry with jitter, fork-join remotes
- **M4 (Job Plane)**: Complete resilience stack, adaptive workers
- **M5 (Privacy)**: Circuit breakers for KMS, cache racing
- **M6-M7 (Verification)**: Observer for metrics, shadow evaluation
- **M8-M9 (Maturity)**: CLI integration, conformance testing
- **M10+ (Enterprise)**: Distributed patterns, security hardening

The roadmaps are tightly coupled, with **Nine Lives P2-P4 being critical for GATOS M3-M5**, and **P5-P7 enabling M6-M9**.
