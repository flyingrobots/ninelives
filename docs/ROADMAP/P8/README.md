# Phase 8: Universal Access (Transports)

**Status:** 📋 Planned

## Executive Summary
*   **Story:** A powerful control plane is useless if you can't reach it. We break down the walls around the `ninelives` control plane by implementing standard transport adapters. This opens the system to the world, allowing standard tools like `curl`, `grpcurl`, or custom dashboards to interact with our resilience policies.
*   **Outcome:** Seamless interoperability. `ninelives` becomes a controllable citizen of the infrastructure, accessible via HTTP REST and gRPC, enabling rich integration with external orchestration and monitoring systems.

## Tasks
- [ ] [P8.01a](P8.01a.md) Wire Format Spec
- [ ] [P8.01b](P8.01b.md) Envelope Serde
- [ ] [P8.01c](P8.01c.md) Command Serde
- [ ] [P8.02a](P8.02a.md) Rest Server Init
- [ ] [P8.02b](P8.02b.md) Rest Auth Middleware
- [ ] [P8.02c](P8.02c.md) Rest Handlers
- [ ] [P8.02d](P8.02d.md) Rest Error Map
- [ ] [P8.02e](P8.02e.md) Rest Example
- [ ] [P8.03a](P8.03a.md) gRPC Protos
- [ ] [P8.03b](P8.03b.md) Tonic Server
- [ ] [P8.03c](P8.03c.md) gRPC Auth
- [ ] [P8.03d](P8.03d.md) gRPC Client
- [ ] [P8.03e](P8.03e.md) gRPC Integ
- [ ] [P8.04a](P8.04a.md) GraphQL Research
- [ ] [P8.04b](P8.04b.md) MCP Research
- [ ] [P8.05a](P8.05a.md) DeadlineContext & Layer
- [ ] [P8.05b](P8.05b.md) Header Extraction (Ingress)
- [ ] [P8.05c](P8.05c.md) Header Injection (Egress)