# Cross-phase edges temporarily removed to finish P2

We removed these DAG edges on 2025-11-28 to allow Phase 2 to close without
waiting on later-phase work. Restore them if we decide P2 should stay
blocked on those deliverables.

- P5.01a -> P2.19   (Sentinel/observer work gating control packaging)
- P6.03b -> P2.19   (Shadow evaluation gating control packaging)
- P7.02a -> P2.19   (Crate split gating control packaging)
- P8.01a -> P2.14b  (HTTP/gRPC transport design gating TransportRouter wrapper)

Rationale: P2’s goal is a working in-process control plane with adaptive
config and basic transports. The above edges point to future phases; keeping
them would prevent declaring P2 complete.
