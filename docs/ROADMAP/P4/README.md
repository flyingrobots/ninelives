# Phase 4: Happy Eyeballs (Fork-Join)

Executive Summary: Implement the "Happy Eyeballs" parallel composition operator (`&`) to allow racing multiple services concurrently, returning the first successful result. This enables low-latency diversity and improved user experience.

## Context

The `&` operator (Bitwise AND) extends the `ninelives` policy algebra to support "Fork-Join" patterns, where two strategies are executed in parallel. This is useful for scenarios like:
- Racing IPv4 and IPv6 connections (traditional "Happy Eyeballs").
- Querying a local cache and a remote database simultaneously.
- Trying multiple redundant upstream services to minimize latency or maximize success rate.

## Tasks
- [/] [P4.01](P4.01.md) **ForkJoinLayer Core Implementation**: Build the fundamental layer for racing two services.
- [/] [P4.02](P4.02.md) **Algebraic Operator Integration**: Integrate the `&` operator into the `Policy` algebra.
- [/] [P4.03](P4.03.md) **Comprehensive Testing & Benchmarking**: Ensure correctness, performance, and resource management.
- [/] [P4.04](P4.04.md) **Documentation & Examples**: Provide clear guidance and practical recipes for usage.

## Alignment with GATOS
- Critical for GATOS M2 (Policy Gate) for racing fast approximate policy evaluations against comprehensive ones.
- Enhances GATOS M3 (Message Bus) for trying multiple Git remotes concurrently.
- Essential for GATOS M5 (Privacy) for racing local cache vs. remote blob fetches.
