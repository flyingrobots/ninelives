# Phase 8: Transport Protocols

Executive Summary: Expand the accessibility of the `ninelives-control` plane by implementing various network transport protocols. This allows external systems (CLIs, UIs, other services) to interact with and manage `ninelives` instances effectively.

## Context

The `ninelives-control` plane, developed in Phase 2, provides an internal API for managing resilience policies. This phase focuses on exposing that API over standard network protocols, leveraging the `Transport` abstraction designed in P2.14.

## Tasks
- [ ] [P8.01](P8.01.md) **Command Serialization & Deserialization**: Define a canonical wire format (e.g., JSON) for `CommandEnvelope` and its components.
- [/] [P8.02](P8.02.md) **HTTP REST Transport**: Implement a `ninelives-rest` crate for HTTP-based control plane access.
- [/] [P8.03](P8.03.md) **gRPC Transport**: Implement a `ninelives-grpc` crate for high-performance, strongly-typed control plane access.
- [/] [P8.04](P8.04.md) **Advanced/Future Transports (MCP/GraphQL)**: Placeholder for specialized or future transport protocols.

## Alignment with GATOS
- **P8.01 (Serialization)** is foundational for `gatos-cli` and `gatos-control-plane` to communicate with `ninelives` instances.
- **P8.02 (HTTP REST)** directly enables GATOS M8 (Demos & Examples) by providing a user-friendly API for `gatos-cli` integration.
- **P8.03 (gRPC)** supports GATOS M10 (Enterprise & Scale) by offering a high-performance control plane suitable for inter-service communication within the GATOS ecosystem.
