# Phase 2

Executive Summary: Enable runtime policy tuning and command execution.

## Tasks
- [ ] [P2.01.a](P2.01.a.md) Design `Adaptive<T>` wrapper: (core implementation)
- [ ] [P2.01.b](P2.01.b.md) Design `Adaptive<T>` wrapper: (tests & docs)
- [ ] [P2.02.a](P2.02.a.md) Integrate Adaptive into policy configs: (core implementation)
- [ ] [P2.02.b](P2.02.b.md) Integrate Adaptive into policy configs: (tests & docs)
- [ ] [P2.03.a](P2.03.a.md) Define `CommandContext` struct: (core implementation)
- [ ] [P2.03.b](P2.03.b.md) Define `CommandContext` struct: (tests & docs)
- [ ] [P2.04.a](P2.04.a.md) Define `CommandHandler` trait as `tower::Service<CommandContext>` (core implementation)
- [ ] [P2.04.b](P2.04.b.md) Define `CommandHandler` trait as `tower::Service<CommandContext>` (tests & docs)
- [ ] [P2.05.a](P2.05.a.md) Implement `ControlPlaneRouter`: (core implementation)
- [ ] [P2.05.b](P2.05.b.md) Implement `ControlPlaneRouter`: (tests & docs)
- [ ] [P2.06.a](P2.06.a.md) `SetParameterHandler` (update Adaptive values) (core implementation)
- [ ] [P2.06.b](P2.06.b.md) `SetParameterHandler` (update Adaptive values) (tests & docs)
- [ ] [P2.07.a](P2.07.a.md) `GetParameterHandler` (read current config) (core implementation)
- [ ] [P2.07.b](P2.07.b.md) `GetParameterHandler` (read current config) (tests & docs)
- [ ] [P2.08.a](P2.08.a.md) `GetStateHandler` (query policy state) (core implementation)
- [ ] [P2.08.b](P2.08.b.md) `GetStateHandler` (query policy state) (tests & docs)
- [ ] [P2.09.a](P2.09.a.md) `ResetCircuitBreakerHandler` (core implementation)
- [ ] [P2.09.b](P2.09.b.md) `ResetCircuitBreakerHandler` (tests & docs)
- [ ] [P2.10.a](P2.10.a.md) `ListPoliciesHandler` (core implementation)
- [ ] [P2.10.b](P2.10.b.md) `ListPoliciesHandler` (tests & docs)
- [ ] [P2.11.a](P2.11.a.md) Start with `ninelives-control` crate (core implementation)
- [ ] [P2.11.b](P2.11.b.md) Start with `ninelives-control` crate (tests & docs)
- [ ] [P2.12.a](P2.12.a.md) Implement local/in-process transport (channels) (core implementation)
- [ ] [P2.12.b](P2.12.b.md) Implement local/in-process transport (channels) (tests & docs)
- [ ] [P2.13.a](P2.13.a.md) Design transport abstraction for future HTTP/gRPC/etc. (core implementation)
- [ ] [P2.13.b](P2.13.b.md) Design transport abstraction for future HTTP/gRPC/etc. (tests & docs)
- [ ] [P2.14.a](P2.14.a.md) Define `AuthorizationLayer` (checks Identity in CommandContext) (core implementation)
- [ ] [P2.14.b](P2.14.b.md) Define `AuthorizationLayer` (checks Identity in CommandContext) (tests & docs)
- [ ] [P2.15.a](P2.15.a.md) Define `AuditLayer` (logs all commands) (core implementation)
- [ ] [P2.15.b](P2.15.b.md) Define `AuditLayer` (logs all commands) (tests & docs)
- [ ] [P2.16.a](P2.16.a.md) Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit) (core implementation)
- [ ] [P2.16.b](P2.16.b.md) Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit) (tests & docs)

## Definition of Ready
- TBD
