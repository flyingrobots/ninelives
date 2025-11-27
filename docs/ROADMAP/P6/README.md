# Phase 6: Shadow Evaluation

Executive Summary: Implement a "shadow evaluation" system that allows new resilience policies and configurations to be tested in production with live traffic, without affecting the primary request path. This enables safe "what-if" analysis and automated, data-driven policy promotion.

## Context

Changing resilience policies in production is risky. Shadowing provides a mechanism to mitigate this risk by running a new policy in parallel with an existing one, observing its hypothetical behavior, and only promoting it to primary if it proves to be stable and beneficial.

## Tasks
- [/] [P6.01](P6.01.md) **Shadow Layer & Adaptive Support**: Implement the core `ShadowLayer` and extend `Adaptive<T>` to manage shadow configurations.
- [/] [P6.02](P6.02.md) **Shadow Telemetry & Observer Integration**: Define and emit `ShadowEvent`s, ensuring they are properly aggregated and queryable via the `TelemetryAggregator`.
- [/] [P6.03](P6.03.md) **Shadow Promotion & Management**: Implement the logic within `ninelives-sentinel` to observe shadow performance and automatically promote stable shadow policies.
- [/] [P6.04](P6.04.md) **Safety Guarantees & Documentation**: Document the safety guarantees, limitations, and operational best practices for shadow evaluation.

## Alignment with GATOS
- **P6.01-P6.04** are critical for GATOS M9 (Conformance Suite), enabling safe validation of new governance policies before deployment.
- The ability to perform "what-if" analysis on resilience policies directly impacts the verifiability and auditability of the GATOS system, reducing operational risk.
