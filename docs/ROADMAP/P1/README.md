# Phase 1

Executive Summary: Build the observability foundation that enables autonomous operation.

## Tasks
- [ ] [P1.01.a](P1.01.a.md) Define `PolicyEvent` enum: (core implementation)
- [ ] [P1.01.b](P1.01.b.md) Define `PolicyEvent` enum: (tests & docs)
- [ ] [P1.02.a](P1.02.a.md) Add event emission to all policy layers: (core implementation)
- [ ] [P1.02.b](P1.02.b.md) Add event emission to all policy layers: (tests & docs)
- [ ] [P1.03.a](P1.03.a.md) Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>` (core implementation)
- [ ] [P1.03.b](P1.03.b.md) Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>` (tests & docs)
- [ ] [P1.04.a](P1.04.a.md) Implement basic sinks: (core implementation)
- [ ] [P1.04.b](P1.04.b.md) Implement basic sinks: (tests & docs)
- [ ] [P1.05.a](P1.05.a.md) Wire policies to accept telemetry sink via `.with_sink()` method (core implementation)
- [ ] [P1.05.b](P1.05.b.md) Wire policies to accept telemetry sink via `.with_sink()` method (tests & docs)
- [ ] [P1.06.a](P1.06.a.md) Implement `MulticastSink` for sending to multiple sinks (multicast to both) (core implementation)
- [ ] [P1.06.b](P1.06.b.md) Implement `MulticastSink` for sending to multiple sinks (multicast to both) (tests & docs)
- [ ] [P1.07.a](P1.07.a.md) Implement `FallbackSink` for fallback on failure (core implementation)
- [ ] [P1.07.b](P1.07.b.md) Implement `FallbackSink` for fallback on failure (tests & docs)
- [ ] [P1.08.a](P1.08.a.md) Add `ComposedSinkError` type for composition errors (core implementation)
- [ ] [P1.08.b](P1.08.b.md) Add `ComposedSinkError` type for composition errors (tests & docs)
- [x] [P1.09](P1.09.md) Document sink composition patterns
- [ ] [P1.10.a](P1.10.a.md) Thread sink through policy constructors/builders via `.with_sink()` method (core implementation)
- [ ] [P1.10.b](P1.10.b.md) Thread sink through policy constructors/builders via `.with_sink()` method (tests & docs)
- [ ] [P1.11.a](P1.11.a.md) Add examples showing telemetry integration: (core implementation)
- [ ] [P1.11.b](P1.11.b.md) Add examples showing telemetry integration: (tests & docs)
- [ ] [P1.12.a](P1.12.a.md) Benchmark overhead of event emission (deferred to Phase 10) (core implementation)
- [ ] [P1.12.b](P1.12.b.md) Benchmark overhead of event emission (deferred to Phase 10) (tests & docs)

## Definition of Ready
- TBD
