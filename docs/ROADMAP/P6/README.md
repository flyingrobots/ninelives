# Phase 6: Shadow Evaluation

**Status:** 📋 Planned

## Executive Summary
*   **Story:** Change is the leading cause of outages. Rolling out a new resilience policy shouldn't be a leap of faith. We introduce "Shadow Mode"—the ability to run a new policy configuration alongside the live one, processing real traffic without affecting the result, to prove it works before it goes live.
*   **Outcome:** Risk-free policy evolution. Operators can "what-if" test aggressive configurations (e.g., tighter timeouts, stricter rate limits) in production, verify their safety via shadow telemetry, and automatically promote them when proven stable.

## Tasks
- [ ] [P6.01a](P6.01a.md) ShadowLayer Struct
- [ ] [P6.01b](P6.01b.md) Adaptive Shadow Support
- [ ] [P6.01c](P6.01c.md) Shadow Isolation
- [ ] [P6.01d](P6.01d.md) ShadowLayer Unit Tests
- [ ] [P6.01e](P6.01e.md) ShadowLayer Integration
- [ ] [P6.02a](P6.02a.md) ShadowEvent Definition
- [ ] [P6.02b](P6.02b.md) Shadow Emission
- [ ] [P6.02c](P6.02c.md) Aggregator Shadow Support
- [ ] [P6.02d](P6.02d.md) Metrics Separation Tests
- [ ] [P6.03a](P6.03a.md) Atomic Swap Logic
- [ ] [P6.03b](P6.03b.md) Promotion Command
- [ ] [P6.03c](P6.03c.md) Promotion Meta-Policy
- [ ] [P6.03d](P6.03d.md) End-to-End Promotion
- [ ] [P6.04a](P6.04a.md) Safety ADR
- [ ] [P6.04b](P6.04b.md) Shadow Cookbook