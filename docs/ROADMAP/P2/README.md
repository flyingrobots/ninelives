# Phase 2

Executive Summary: Enable runtime policy tuning and command execution.

## Tasks
- [ ] [P2.01](P2.01.md) Design `Adaptive<T>` wrapper:
- [ ] [P2.01.a](P2.01.a.md) Design `Adaptive<T>` wrapper:
- [ ] [P2.01.b](P2.01.b.md) Design `Adaptive<T>` wrapper:
- [ ] [P2.02](P2.02.md) Integrate Adaptive into policy configs:
- [ ] [P2.02.a](P2.02.a.md) Integrate Adaptive into policy configs:
- [ ] [P2.02.b](P2.02.b.md) Integrate Adaptive into policy configs:
- [ ] [P2.03](P2.03.md) Define `CommandContext` struct:
- [ ] [P2.03.a](P2.03.a.md) Define `CommandContext` struct:
- [ ] [P2.03.b](P2.03.b.md) Define `CommandContext` struct:
- [ ] [P2.04](P2.04.md) Define `CommandHandler` trait as `tower::Service<CommandContext>`
- [ ] [P2.04.a](P2.04.a.md) Define `CommandHandler` trait as `tower::Service<CommandContext>`
- [ ] [P2.04.b](P2.04.b.md) Define `CommandHandler` trait as `tower::Service<CommandContext>`
- [ ] [P2.05](P2.05.md) Implement `ControlPlaneRouter`:
- [ ] [P2.05.a](P2.05.a.md) Implement `ControlPlaneRouter`:
- [ ] [P2.05.b](P2.05.b.md) Implement `ControlPlaneRouter`:
- [ ] [P2.06](P2.06.md) `SetParameterHandler` (update Adaptive values)
- [ ] [P2.06.a](P2.06.a.md) `SetParameterHandler` (update Adaptive values)
- [ ] [P2.06.b](P2.06.b.md) `SetParameterHandler` (update Adaptive values)
- [ ] [P2.07](P2.07.md) `GetParameterHandler` (read current config)
- [ ] [P2.07.a](P2.07.a.md) `GetParameterHandler` (read current config)
- [ ] [P2.07.b](P2.07.b.md) `GetParameterHandler` (read current config)
- [ ] [P2.08](P2.08.md) `GetStateHandler` (query policy state)
- [ ] [P2.08.a](P2.08.a.md) `GetStateHandler` (query policy state)
- [ ] [P2.08.b](P2.08.b.md) `GetStateHandler` (query policy state)
- [ ] [P2.09](P2.09.md) `ResetCircuitBreakerHandler`
- [ ] [P2.09.a](P2.09.a.md) `ResetCircuitBreakerHandler`
- [ ] [P2.09.b](P2.09.b.md) `ResetCircuitBreakerHandler`
- [ ] [P2.10](P2.10.md) `ListPoliciesHandler`
- [ ] [P2.10.a](P2.10.a.md) `ListPoliciesHandler`
- [ ] [P2.10.b](P2.10.b.md) `ListPoliciesHandler`
- [ ] [P2.11](P2.11.md) Start with `ninelives-control` crate
- [ ] [P2.11.a](P2.11.a.md) Start with `ninelives-control` crate
- [ ] [P2.11.b](P2.11.b.md) Start with `ninelives-control` crate
- [ ] [P2.12](P2.12.md) Implement local/in-process transport (channels)
- [ ] [P2.12.a](P2.12.a.md) Implement local/in-process transport (channels)
- [ ] [P2.12.b](P2.12.b.md) Implement local/in-process transport (channels)
- [ ] [P2.13](P2.13.md) Design transport abstraction for future HTTP/gRPC/etc.
- [ ] [P2.13.a](P2.13.a.md) Design transport abstraction for future HTTP/gRPC/etc.
- [ ] [P2.13.b](P2.13.b.md) Design transport abstraction for future HTTP/gRPC/etc.
- [ ] [P2.14](P2.14.md) Define `AuthorizationLayer` (checks Identity in CommandContext)
- [ ] [P2.14.a](P2.14.a.md) Define `AuthorizationLayer` (checks Identity in CommandContext)
- [ ] [P2.14.b](P2.14.b.md) Define `AuthorizationLayer` (checks Identity in CommandContext)
- [ ] [P2.15](P2.15.md) Define `AuditLayer` (logs all commands)
- [ ] [P2.15.a](P2.15.a.md) Define `AuditLayer` (logs all commands)
- [ ] [P2.15.b](P2.15.b.md) Define `AuditLayer` (logs all commands)
- [ ] [P2.16](P2.16.md) Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)
- [ ] [P2.16.a](P2.16.a.md) Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)
- [ ] [P2.16.b](P2.16.b.md) Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)

## Definition of Ready
- TBD
