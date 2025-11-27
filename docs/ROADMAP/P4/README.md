# Phase 4: Happy Eyeballs (Fork-Join)

**Status:** 📋 Planned

## Executive Summary
*   **Story:** In a distributed world, redundancy is common but often underutilized. Why wait for a primary to fail before trying a backup? This phase introduces the power of parallelism to the policy algebra, enabling "Happy Eyeballs" patterns where we race multiple paths simultaneously and take the winner.
*   **Outcome:** A new `&` operator for the policy algebra that enables significant latency reductions and higher availability by masking tail latency and individual node failures through concurrent execution.

## Tasks
- [ ] [P4.01a](P4.01a.md) ForkJoinService Logic
- [ ] [P4.01b](P4.01b.md) ForkJoin Cancellation
- [ ] [P4.02a](P4.02a.md) BitAnd Operator
- [ ] [P4.03a](P4.03a.md) Race Tests
- [ ] [P4.03b](P4.03b.md) Cancellation Tests
- [ ] [P4.04a](P4.04a.md) ForkJoin Docs
- [ ] [P4.04b](P4.04b.md) ForkJoin Cookbook
