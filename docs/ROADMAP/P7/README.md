# Phase 7: Crate Split

Executive Summary: Refactor the monolithic `ninelives` crate into a well-defined Cargo workspace with smaller, focused crates. This enhances modularity, improves build times, and clarifies architectural boundaries, making `ninelives` easier to maintain, extend, and use.

## Context

As `ninelives` grows, a single large crate can become unwieldy. Splitting into `ninelives-core`, `ninelives-control`, `ninelives-observer`, and `ninelives-sentinel` (from P5) allows for more granular dependency management, reduces compilation times for specific use cases, and aligns with best practices for larger Rust projects. The original `ninelives` crate will become a meta-crate that re-exports components for convenience and backward compatibility.

## Tasks
- [/] [P7.01](P7.01.md) **Workspace Setup & Core Crate Split**: Convert to a Cargo workspace and extract core resilience primitives into `ninelives-core`.
- [/] [P7.02](P7.02.md) **Control & Observer Crates Split**: Extract control plane components into `ninelives-control` and telemetry/adaptive logic into `ninelives-observer`.
- [/] [P7.03](P7.03.md) **Compatibility & Adapter Development Guidance**: Ensure backward compatibility and provide clear guidance for external developers to build extensions.

## Alignment with GATOS
- **P7.01-P7.03** are crucial for GATOS by providing a cleaner, more modular `ninelives` dependency structure. This allows GATOS components to import only the necessary parts of `ninelives` (e.g., `gatos-policy` might only need `ninelives-core` for algebra and specific layers, `gatos-control` would need `ninelives-control`).
- This phase directly enables better dependency management and reduces the binary size of GATOS components.
