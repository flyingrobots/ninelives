# Phase 3: Adaptive Policies

**Status:** 📋 Planned

## Executive Summary
*   **Story:** Manual tuning doesn't scale. To truly solve the resilience problem, we must connect the system's "eyes" (telemetry) directly to its "hands" (control plane) to create fast, autonomous reflexes. This phase closes the loop, turning static policies into dynamic, self-regulating agents.
*   **Outcome:** A suite of "smart" policies—like AIMD concurrency limits, Rate Limiters, and Retry Budgets—that automatically adapt to changing load conditions to maximize throughput and protect downstream services.

## Tasks
- [ ] [P3.01a](P3.01a.md) Sliding Window Structure
- [ ] [P3.01b](P3.01b.md) Window Statistics
- [ ] [P3.02a](P3.02a.md) Aggregator Storage
- [ ] [P3.02b](P3.02b.md) TelemetrySink Implementation
- [ ] [P3.03a](P3.03a.md) ControlLaw Trait
- [ ] [P3.03b](P3.03b.md) Feedback Loop Runner
- [ ] [P3.04a](P3.04a.md) AIMD Logic
- [ ] [P3.04b](P3.04b.md) AIMD Integration
- [ ] [P3.05a](P3.05a.md) Retry Budget Logic
- [ ] [P3.05b](P3.05b.md) Retry Budget Integration
- [ ] [P3.06](P3.06.md) System State Query Interface
- [ ] [P3.07a](P3.07a.md) RateLimitLayer Struct
- [ ] [P3.07b](P3.07b.md) Adaptive Quota Integration
- [ ] [P3.07c](P3.07c.md) Rate Limiting Recipe