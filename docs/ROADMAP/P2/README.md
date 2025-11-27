# Phase 2: The Control Plane

**Status:** 🚧 In Progress

## Executive Summary
*   **Story:** Static configuration is brittle in dynamic environments. To operate reliable systems, we need "knobs and levers" that can be adjusted at runtime without restarting the application. This phase builds the command-and-control infrastructure that allows operators (and eventually the Sentinel) to tune policy parameters live.
*   **Outcome:** A secure, interactive control plane where `Adaptive<T>` configuration handles can be inspected and modified on the fly via a standardized command interface.

## Tasks
- [x] [P2.01](P2.01.md) Define `Adaptive<T>` types
- [x] [P2.02](P2.02.md) Implement lock-free `arc-swap` for Adaptive
- [x] [P2.03](P2.03.md) Integrate Adaptive into `CircuitBreaker`
- [x] [P2.04](P2.04.md) Integrate Adaptive into `Bulkhead`
- [x] [P2.05](P2.05.md) Integrate Adaptive into `Retry`
- [x] [P2.06](P2.06.md) Integrate Adaptive into `Timeout`
- [x] [P2.07](P2.07.md) Define `Command` enum and `CommandHandler` trait
- [x] [P2.08](P2.08.md) Implement `CommandRouter`
- [x] [P2.09](P2.09.md) Create `ConfigRegistry` for adaptive handles
- [x] [P2.10](P2.10.md) Implement `Set/Get` commands
- [ ] [P2.11](P2.11.md) System State Query (Debug)
- [x] [P2.12](P2.12.md) ResetCircuitBreaker handler
- [x] [P2.121](P2.121.md) Circuit breaker registry + ids
- [x] [P2.13](P2.13.md) ListConfig handler
- [ ] [P2.14a](P2.14a.md) Transport Trait & Envelope
- [ ] [P2.14b](P2.14b.md) TransportRouter Wrapper
- [/] [P2.15](P2.15.md) In-process transport adapter
- [ ] [P2.16a](P2.16a.md) AuthorizationLayer Structure
- [ ] [P2.16b](P2.16b.md) Authorization Logic Integration
- [x] [P2.17](P2.17.md) Audit logging integration
- [/] [P2.18](P2.18.md) Router wrapping with AuthZ + Audit policies
- [/] [P2.19](P2.19.md) Package `ninelives-control`
- [/] [P2.20](P2.20.md) Cookbook examples for control plane
