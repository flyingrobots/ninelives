# Phase 5: Self-Healing Brain (Sentinel)

Executive Summary: Build `ninelives-sentinel`, a meta-policy engine capable of observing system state, evaluating scripts, and issuing commands to achieve autonomous self-healing capabilities. This is the "brain" of the entire `ninelives` ecosystem.

## Context

Phase 3 provides the observational capabilities (metrics aggregation). Phase 5 uses this observation as input for a decision-making engine that can then issue commands back into the system (via the control plane from Phase 2). The goal is to move from manual configuration to automated, adaptive resilience.

## Tasks
- [ ] [P5.01](P5.01.md) **Sentinel Crate & Top-Level Coordinator**: Establish the `ninelives-sentinel` crate and its main orchestration logic.
- [/] [P5.02](P5.02.md) **Scripting Engine Integration & API**: Integrate an embedded scripting language (e.g., Rhai) and define a safe, sandboxed API for scripts to interact with `ninelives` components.
- [/] [P5.03](P5.03.md) **Meta-Policy Evaluation Loop & Management**: Implement the continuous loop for executing meta-policy scripts and provide dynamic script loading/reloading capabilities.
- [/] [P5.04](P5.04.md) **Built-in Meta-Policies & Examples**: Translate P3's adaptive control laws into concrete example meta-policy scripts, demonstrating automated self-tuning.

## Alignment with GATOS
- **P5.01-P5.04** are critical for GATOS M6 (Explorer & Verification) and M7 (Proof-of-Experiment) by enabling verifiable autonomous operation.
- The `Sentinel` will be the primary mechanism for GATOS to achieve self-tuning worker pools, adaptive policy gates, and automated incident response based on observed conditions.
