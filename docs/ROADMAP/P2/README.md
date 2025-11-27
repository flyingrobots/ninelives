# Phase 2

Executive Summary: Enable runtime policy tuning and command execution.

## Task DAG
See [DAG.csv](DAG.csv) for the edge list.

## Workstreams (topological within each)
### Adaptive Core & Integrations
- [ ] [P2.01](P2.01.md) Adaptive core: API + ArcSwap default
- [/] [P2.02](P2.02.md) Adaptive RwLock feature flag parity
- [/] [P2.03](P2.03.md) Adaptive integration: Retry max_attempts/backoff/jitter
- [/] [P2.04](P2.04.md) Adaptive integration: Timeout duration
- [/] [P2.05](P2.05.md) Adaptive integration: CircuitBreaker thresholds/timeouts
- [/] [P2.06](P2.06.md) Adaptive integration: Bulkhead max_concurrent

### Command Schema & Router
- [ ] [P2.07](P2.07.md) CommandContext schema (id, args, identity, response channel placeholder)
- [/] [P2.08](P2.08.md) CommandHandler trait (tower::Service<CommandContext>)
- [/] [P2.09](P2.09.md) ControlPlaneRouter skeleton (auth, dispatch, history)

### Parameter/State Handlers
- [/] [P2.10](P2.10.md) Parameter handlers: Set/Get adaptive values
- [/] [P2.11](P2.11.md) State handler: query policy state
- [/] [P2.12](P2.12.md) ResetCircuitBreaker handler
- [/] [P2.13](P2.13.md) ListPolicies handler

### Transport
- [/] [P2.14](P2.14.md) Transport abstraction design (HTTP/gRPC friendly)
- [/] [P2.15](P2.15.md) In-process transport (channels)

### Auth/Audit/Wrap
- [/] [P2.16](P2.16.md) Authorization layer
- [/] [P2.17](P2.17.md) Audit layer
- [/] [P2.18](P2.18.md) Router wrapping with AuthZ + Audit policies

### Packaging & Docs
- [/] [P2.19](P2.19.md) Package ninelives-control crate
- [/] [P2.20](P2.20.md) Docs + examples for control plane

## Tasks (topological order)
- [ ] [P2.01](P2.01.md) Adaptive core: API + ArcSwap default
- [ ] [P2.07](P2.07.md) CommandContext schema (id, args, identity, response channel placeholder)
- [/] [P2.02](P2.02.md) Adaptive RwLock feature flag parity
- [/] [P2.03](P2.03.md) Adaptive integration: Retry max_attempts/backoff/jitter
- [/] [P2.04](P2.04.md) Adaptive integration: Timeout duration
- [/] [P2.05](P2.05.md) Adaptive integration: CircuitBreaker thresholds/timeouts
- [/] [P2.06](P2.06.md) Adaptive integration: Bulkhead max_concurrent
- [/] [P2.08](P2.08.md) CommandHandler trait (tower::Service<CommandContext>)
- [/] [P2.09](P2.09.md) ControlPlaneRouter skeleton (auth, dispatch, history)
- [/] [P2.10](P2.10.md) Parameter handlers: Set/Get adaptive values
- [/] [P2.11](P2.11.md) State handler: query policy state
- [/] [P2.12](P2.12.md) ResetCircuitBreaker handler
- [/] [P2.13](P2.13.md) ListPolicies handler
- [/] [P2.14](P2.14.md) Transport abstraction design (HTTP/gRPC friendly)
- [/] [P2.16](P2.16.md) Authorization layer
- [/] [P2.17](P2.17.md) Audit layer
- [/] [P2.15](P2.15.md) In-process transport (channels)
- [/] [P2.18](P2.18.md) Router wrapping with AuthZ + Audit policies
- [/] [P2.19](P2.19.md) Package ninelives-control crate
- [/] [P2.20](P2.20.md) Docs + examples for control plane

## Definition of Ready
- TBD
