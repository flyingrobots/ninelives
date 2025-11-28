# Phase 4: Happy Eyeballs (Fork-Join)

**Status:** 📋 Planned

## Executive Summary
*   **Story:** In a distributed world, redundancy is common but often underutilized. Why wait for a primary to fail before trying a backup? This phase introduces the power of parallelism to the policy algebra, enabling "Happy Eyeballs" patterns where we race multiple paths simultaneously and take the winner.
*   **Outcome:** A new `&` operator for the policy algebra that enables significant latency reductions and higher availability by masking tail latency and individual node failures through concurrent execution.

## Tasks

### P4.01a ForkJoinService Logic

| field | value |
| --- | --- |
| id | P4.01a |
| title | ForkJoinService Logic |
| estimate | 2h |
| status | open |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement `ForkJoinService` struct and `Future` polling logic to race two services.

#### Steps
1.  Define `ForkJoinService<A, B>`.
2.  Implement `Service::call`: Spawn both futures using `futures::future::select`.
3.  Return result of winner.

#### Definition of Done
- [ ] Service runs two futures.
- [ ] Returns first success.

### P4.01b ForkJoin Cancellation

| field | value |
| --- | --- |
| id | P4.01b |
| title | ForkJoin Cancellation |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Ensure the losing future in `ForkJoinService` is cancelled (dropped) immediately.

#### Steps
1.  Verify `select` drops the other future.
2.  Handle error aggregation if both fail (wait for second failure).

#### Definition of Done
- [ ] Losing future cancelled.
- [ ] Both-fail returns combined error.

### P4.02a BitAnd Operator

| field | value |
| --- | --- |
| id | P4.02a |
| title | BitAnd Operator |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Implement `std::ops::BitAnd` for `Policy<L>`.

#### Steps
1.  Impl `BitAnd` for `Policy`.
2.  Return `Policy<ForkJoinLayer<...>>`.

#### Definition of Done
- [ ] `policy_a & policy_b` compiles.

### P4.03a Race Tests

| field | value |
| --- | --- |
| id | P4.03a |
| title | Race Tests |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Unit tests for racing behavior.

#### Steps
1.  Test A faster than B.
2.  Test B faster than A.
3.  Test success vs failure racing.

#### Definition of Done
- [ ] Race conditions covered.

### P4.03b Cancellation Tests

| field | value |
| --- | --- |
| id | P4.03b |
| title | Cancellation Tests |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Verify cancellation logic.

#### Steps
1.  Mock service that flags on drop.
2.  Assert flag set when other service wins.

#### Definition of Done
- [ ] Cancellation verified.

### P4.04a ForkJoin Docs

| field | value |
| --- | --- |
| id | P4.04a |
| title | ForkJoin Docs |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Document `ForkJoinLayer` and `&` operator.

#### Steps
1.  Update `algebra.rs` docs.
2.  Update `README.md`.

#### Definition of Done
- [ ] Documentation complete.

### P4.04b ForkJoin Cookbook

| field | value |
| --- | --- |
| id | P4.04b |
| title | ForkJoin Cookbook |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Add "Happy Eyeballs" recipe.

#### Steps
1.  Create `examples/happy_eyeballs.rs`.
2.  Simulate IPv4/IPv6 race.

#### Definition of Done
- [ ] Recipe compiles and runs.
