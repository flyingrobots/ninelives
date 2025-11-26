# Phase 4

Executive Summary: Implement the "happy eyeballs" parallel composition operator.

## Tasks
- [ ] [P4.01.a](P4.01.a.md) Design `ForkJoinLayer` and `ForkJoinService` (core implementation)
- [ ] [P4.01.b](P4.01.b.md) Design `ForkJoinLayer` and `ForkJoinService` (tests & docs)
- [ ] [P4.02.a](P4.02.a.md) Spawn both services concurrently (futures::select for racing) (core implementation)
- [ ] [P4.02.b](P4.02.b.md) Spawn both services concurrently (futures::select for racing) (tests & docs)
- [ ] [P4.03.a](P4.03.a.md) Return first `Ok` result (core implementation)
- [ ] [P4.03.b](P4.03.b.md) Return first `Ok` result (tests & docs)
- [ ] [P4.04.a](P4.04.a.md) Cancel remaining futures on first success (core implementation)
- [ ] [P4.04.b](P4.04.b.md) Cancel remaining futures on first success (tests & docs)
- [ ] [P4.05.a](P4.05.a.md) Handle case where both fail (return error) (core implementation)
- [ ] [P4.05.b](P4.05.b.md) Handle case where both fail (return error) (tests & docs)
- [ ] [P4.06.a](P4.06.a.md) Implement `BitAnd` trait for `Policy<L>` (core implementation)
- [ ] [P4.06.b](P4.06.b.md) Implement `BitAnd` trait for `Policy<L>` (tests & docs)
- [ ] [P4.07.a](P4.07.a.md) Returns `Policy<ForkJoinLayer<A, B>>` (core implementation)
- [ ] [P4.07.b](P4.07.b.md) Returns `Policy<ForkJoinLayer<A, B>>` (tests & docs)
- [ ] [P4.08](P4.08.md) Test race conditions (doc tests cover both sides)
- [ ] [P4.09.a](P4.09.a.md) Test both-fail scenarios (implemented in service logic) (core implementation)
- [ ] [P4.09.b](P4.09.b.md) Test both-fail scenarios (implemented in service logic) (tests & docs)
- [ ] [P4.10.a](P4.10.a.md) Test cancellation behavior (futures::select handles drop) (core implementation)
- [ ] [P4.10.b](P4.10.b.md) Test cancellation behavior (futures::select handles drop) (tests & docs)
- [ ] [P4.11.a](P4.11.a.md) Benchmark overhead vs sequential (core implementation)
- [ ] [P4.11.b](P4.11.b.md) Benchmark overhead vs sequential (tests & docs)
- [ ] [P4.12.a](P4.12.a.md) Add examples: IPv4/IPv6, cache strategies (core implementation)
- [ ] [P4.12.b](P4.12.b.md) Add examples: IPv4/IPv6, cache strategies (tests & docs)
- [ ] [P4.13](P4.13.md) Document operator precedence: `&` > `+` > `|`
- [ ] [P4.14.a](P4.14.a.md) Add to algebra guide (README, lib.rs, examples) (core implementation)
- [ ] [P4.14.b](P4.14.b.md) Add to algebra guide (README, lib.rs, examples) (tests & docs)

## Definition of Ready
- TBD
