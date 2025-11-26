# Phase 6

Executive Summary: Enable safe what-if analysis before applying policy changes.

## Tasks
- [ ] [P6.01.a](P6.01.a.md) Extend Adaptive<T> to support shadow values: (core implementation)
- [ ] [P6.01.b](P6.01.b.md) Extend Adaptive<T> to support shadow values: (tests & docs)
- [ ] [P6.02.a](P6.02.a.md) Add shadow mode to policy layers: (core implementation)
- [ ] [P6.02.b](P6.02.b.md) Add shadow mode to policy layers: (tests & docs)
- [ ] [P6.03.a](P6.03.a.md) Define `ShadowEvent` type (includes primary + shadow results) (core implementation)
- [ ] [P6.03.b](P6.03.b.md) Define `ShadowEvent` type (includes primary + shadow results) (tests & docs)
- [ ] [P6.04.a](P6.04.a.md) Add shadow event emission to policies (core implementation)
- [ ] [P6.04.b](P6.04.b.md) Add shadow event emission to policies (tests & docs)
- [ ] [P6.05.a](P6.05.a.md) Observer ingests shadow events separately (core implementation)
- [ ] [P6.05.b](P6.05.b.md) Observer ingests shadow events separately (tests & docs)
- [ ] [P6.06.a](P6.06.a.md) Sentinel observes shadow stability over time window (core implementation)
- [ ] [P6.06.b](P6.06.b.md) Sentinel observes shadow stability over time window (tests & docs)
- [ ] [P6.07.a](P6.07.a.md) Issues `PromoteShadowHandler` command if stable (core implementation)
- [ ] [P6.07.b](P6.07.b.md) Issues `PromoteShadowHandler` command if stable (tests & docs)
- [ ] [P6.08.a](P6.08.a.md) Policies atomically swap shadow -> primary (core implementation)
- [ ] [P6.08.b](P6.08.b.md) Policies atomically swap shadow -> primary (tests & docs)
- [ ] [P6.09.a](P6.09.a.md) Ensure shadow evaluation doesn't affect primary path latency (core implementation)
- [ ] [P6.09.b](P6.09.b.md) Ensure shadow evaluation doesn't affect primary path latency (tests & docs)
- [ ] [P6.10.a](P6.10.a.md) Add circuit breaker to kill shadow eval if too expensive (core implementation)
- [ ] [P6.10.b](P6.10.b.md) Add circuit breaker to kill shadow eval if too expensive (tests & docs)
- [ ] [P6.11](P6.11.md) Document safety guarantees

## Definition of Ready
- TBD
