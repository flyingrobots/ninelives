# v0.3.0 Roadmap: The Control Plane & Production Hardening

This roadmap focuses on graduating the Control Plane from "experimental" to "production-ready" by addressing critical stability risks, improving developer experience (DX) via presets, and optimizing core performance.

## 🚨 P0: Critical Stability & Safety
*Must be completed before v0.3.0 release.*

- [x] **Fix: Unbounded MemoryAuditSink**
  - **Context:** The default audit sink has no capacity limit, posing a DOS/OOM risk in production.
  - **Task:** Implement a ring buffer (circular buffer) or simple capacity cap (evict oldest).
  - **Estimate:** 1h / ~30 LoC
  - **Source:** `ignore.SHIPREADY.md`

- [x] **Feat: Production Presets (`src/presets.rs`)**
  - **Context:** Users currently manually compose 4+ layers, leading to misconfigurations (e.g., missing bulkheads).
  - **Task:** Export `web_service`, `database_client`, `external_api`, `fast_cache`, `message_producer`.
  - **Estimate:** 2h / ~200 LoC
  - **Source:** `ignore.SHIPREADY.md`

## ⚡ P1: Core Performance
*Optimization to ensure low-overhead resilience.*

- [x] **Perf: Zero-Allocation CircuitBreaker**
  - **Context:** `Box<Pin<...>>` in `CircuitBreakerService` causes heap allocation on *every* request.
  - **Task:** Refactor to use `pin-project` for a custom Future type.
  - **Estimate:** 4h / ~100 LoC (High Complexity)
  - **Source:** `ignore.Weaknesses.md`

## 🏗 P2: Architecture & Quality (Tier 2)
*Structural improvements to support scale and testing.*

- [x] **Feat: Abstract RateLimiter Interface**
  - **Context:** Enabling distributed rate limiting (e.g., Redis-backed) requires decoupling logic from state.
  - **Task:** Define `RateLimiter` trait and `TokenBucket` abstractions. Create `RateLimitLayer<L>`.
  - **Estimate:** 3h
  - **Goal:** "Lego-block-ibility" for distributed state.

- [x] **Refactor: Telemetry Module**
  - **Context:** `src/telemetry.rs` is a monolithic file.
  - **Task:** Split into `telemetry/events.rs`, `telemetry/sinks.rs`, `telemetry/context.rs`.
  - **Estimate:** 2h

- [x] **Feat: Dependency Injection for Testing**
  - **Context:** Hard to test policies deterministically.
  - **Task:** Introduce `MockClock` and `RecordingSink` traits/structs accessible to users.
  - **Estimate:** 3h

- [ ] **Refactor: Extensible Control Plane**
  - **Context:** Command types are currently hardcoded enums.
  - **Task:** Define a `Command` trait to allow users to register custom admin commands.
  - **Estimate:** 4h

## 📚 P3: Documentation & Operations
*Empowering the "Lego Block" philosophy.*

- [ ] **Docs: Operations Guide (`docs/OPERATIONS.md`)**
  - **Context:** No guidance on monitoring, capacity planning, or incident response.
  - **Task:** Create guide covering "How to monitor," "What to alert on," and "Emergency Manual Overrides."
  - **Estimate:** 3h
  - **Source:** `ignore.DocsReport.md`

- [ ] **Example: Persistence Patterns**
  - **Context:** Users need to see *how* to implement the "lego block" persistence.
  - **Task:** Add `ninelives-cookbook/examples/persistent_state.rs` showing snapshot/restore of the Control Plane.
  - **Estimate:** 2h

## 🧹 P4: Hygiene & Tier 3 (Nice-to-Haves)
*Items we are consciously deprioritizing for now but tracking.*

- [ ] **CI: Dependency Management**
  - **Task:** Configure `dependabot.yml` and add `cargo audit` to CI.
  - **Estimate:** 1h
  - **Source:** `ignore.SHIPREADY.md`

- [ ] **Refactor: Rename `retry_attempts`**
  - **Task:** Rename to `max_attempts` for clarity/consistency.
  - **Estimate:** 1h

- [ ] **Feat: Dynamic Bulkhead Resizing** (Tier 3)
  - **Context:** Bulkheads are currently fixed-size.
  - **Verdict:** Deferred. Adaptive bulkheads (TCP Vegas style) are complex and belong in v0.4.0.

- [ ] **Feat: Redis/Etcd Transport Adapters** (Tier 3)
  - **Context:** Native adapters for the control plane.
  - **Verdict:** Deferred. The `cookbook` examples are sufficient for v0.3.0.
