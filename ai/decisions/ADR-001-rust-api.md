# ADR-001: Rust & Axum for High-Assurance Backend

## Status
Accepted

## Context
Equity Catalyst requires an off-chain orchestration service capable of:
- Evaluating complex mathematical policies and risk parameters across continuous streams of market events.
- Interacting directly with the Solana blockchain via native SDK types without serialization friction.
- Guaranteeing memory safety, zero data races, and strict non-panicking financial arithmetic.
- Maintaining high throughput with minimal, predictable latency under burst market volatility.

## Decision
We build the core backend API and background workers in **Rust** using:
- **Axum 0.7** as the modular HTTP framework built on Tower and Hyper.
- **Tokio** as the multi-threaded asynchronous runtime.
- **deadpool-postgres & tokio-postgres** for pooled, async relational persistence.
- **solana-sdk & solana-client** for native RPC, transaction building, and WebSocket subscriptions.

## Consequences
### Positive
- Strict compile-time safety and absence of runtime null-pointer/type crashes.
- Direct code sharing between off-chain services and on-chain Anchor types via `crates/shared`.
- Zero-garbage-collection latency profiles essential during risk stop-loss liquidations.

### Negative
- Slower compilation times compared to dynamic scripting runtimes.
- Requires higher developer discipline in managing asynchronous ownership and lifetimes.
