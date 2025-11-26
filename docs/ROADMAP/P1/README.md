# Phase 1

Executive Summary: Build the observability foundation that enables autonomous operation.

## Tasks
- [ ] [P1.01.a](P1.01.a.md) Define `PolicyEvent` enum:
- [ ] [P1.01.b](P1.01.b.md) Define `PolicyEvent` enum:
- [ ] [P1.02.a](P1.02.a.md) Add event emission to all policy layers:
- [ ] [P1.02.b](P1.02.b.md) Add event emission to all policy layers:
- [ ] [P1.03.a](P1.03.a.md) Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
- [ ] [P1.03.b](P1.03.b.md) Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
- [ ] [P1.04.a](P1.04.a.md) Implement basic sinks:
- [ ] [P1.04.b](P1.04.b.md) Implement basic sinks:
- [ ] [P1.05.a](P1.05.a.md) Wire policies to accept telemetry sink via `.with_sink()` method
- [ ] [P1.05.b](P1.05.b.md) Wire policies to accept telemetry sink via `.with_sink()` method
- [ ] [P1.06.a](P1.06.a.md) Implement `MulticastSink` for sending to multiple sinks (multicast to both)
- [ ] [P1.06.b](P1.06.b.md) Implement `MulticastSink` for sending to multiple sinks (multicast to both)
- [ ] [P1.07.a](P1.07.a.md) Implement `FallbackSink` for fallback on failure
- [ ] [P1.07.b](P1.07.b.md) Implement `FallbackSink` for fallback on failure
- [ ] [P1.08.a](P1.08.a.md) Add `ComposedSinkError` type for composition errors
- [ ] [P1.08.b](P1.08.b.md) Add `ComposedSinkError` type for composition errors
- [ ] [P1.09](P1.09.md) Document sink composition patterns
- [ ] [P1.10.a](P1.10.a.md) Thread sink through policy constructors/builders via `.with_sink()` method
- [ ] [P1.10.b](P1.10.b.md) Thread sink through policy constructors/builders via `.with_sink()` method
- [ ] [P1.11.a](P1.11.a.md) Add examples showing telemetry integration:
- [ ] [P1.11.b](P1.11.b.md) Add examples showing telemetry integration:
- [ ] [P1.12.a](P1.12.a.md) Benchmark overhead of event emission (deferred to Phase 10)
- [ ] [P1.12.b](P1.12.b.md) Benchmark overhead of event emission (deferred to Phase 10)

## Definition of Ready
- TBD
