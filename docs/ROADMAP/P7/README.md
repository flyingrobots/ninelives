# Phase 7: Modular Ecosystem

**Status:** 📋 Planned

## Executive Summary
*   **Story:** One size does not fit all. A monolithic library creates bloat and forces unnecessary dependencies. We refactor `ninelives` into a composable ecosystem of crates, allowing users to pick exactly what they need—whether it's just the core primitives, the full control plane, or specialized adapters.
*   **Outcome:** A flexible, lightweight architecture with a minimal `ninelives-core` suitable for everything from embedded devices to massive microservices, supported by a rich ecosystem of optional extensions.

## Tasks
- [ ] [P7.01a](P7.01a.md) Workspace Setup
- [ ] [P7.01b](P7.01b.md) Extract Primitives
- [ ] [P7.01c](P7.01c.md) Extract Layers
- [ ] [P7.01d](P7.01d.md) Meta-Crate Setup
- [ ] [P7.01e](P7.01e.md) Cookbook Fixes
- [ ] [P7.01f](P7.01f.md) CI Updates
- [ ] [P7.02a](P7.02a.md) Extract Control Crate
- [ ] [P7.02b](P7.02b.md) Extract Observer Crate
- [ ] [P7.02c](P7.02c.md) Move Sentinel Crate
- [ ] [P7.02d](P7.02d.md) Re-export Cleanup
- [ ] [P7.03a](P7.03a.md) Adapter Guide
- [ ] [P7.03b](P7.03b.md) Adapter Template
- [ ] [P7.04a](P7.04a.md) CoalesceLayer Logic
- [ ] [P7.04b](P7.04b.md) Shared Future Implementation
- [ ] [P7.04c](P7.04c.md) Coalescing Tests