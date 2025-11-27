# Phase 3: Adaptive Policies

Executive Summary: Transform `ninelives` from a static configuration library into a dynamic, self-regulating system.

This phase builds the **Closed Loop Control** capabilities:
1.  **Observe**: Aggregate raw events into windowed metrics (Rates, Latencies).
2.  **Decide**: Use Control Laws (AIMD, Budgets) to calculate optimal parameters.
3.  **Act**: Update `Adaptive<T>` handles in real-time.

## Tasks
- [ ] [P3.01](P3.01.md) **Windowed Metrics Primitives**: The core data structures.
- [/] [P3.02](P3.02.md) **Telemetry Aggregator**: The sink that fills the windows.
- [/] [P3.03](P3.03.md) **Adaptive Controller Primitive**: The generic feedback loop runner.
- [/] [P3.04](P3.04.md) **AIMD Concurrency**: Auto-scaling for Bulkheads.
- [/] [P3.05](P3.05.md) **Retry Budgeting**: Storm prevention.
- [/] [P3.06](P3.06.md) **System State Query**: Exposing the metrics.

## Alignment with GATOS
- **P3.04 (AIMD)** is critical for GATOS M4 (Job Plane) to auto-scale worker pools.
- **P3.05 (Retry Budget)** is critical for GATOS M3 (Message Plane) to prevent Git CAS retry storms.
