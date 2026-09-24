# Canonical Implementation Order

This document defines the strict, authoritative 19-phase sequence for Equity Catalyst development.

```
PHASE 0 — Baseline & Safety
        ↓
PHASE 1 — Canonical Asset Registry
        ↓
PHASE 2 — Asset ↔ Pyth Feed Mapping
        ↓
PHASE 3 — Dynamic Subscription Model
        ↓
PHASE 4 — Pyth Real-Time SSE Client
        ↓
PHASE 5 — Dynamic Stream Manager
        ↓
PHASE 6 — Market Data Store
        ↓
PHASE 7 — Oracle Validation
        ↓
PHASE 8 — REST + WebSocket API
        ↓
PHASE 9 — DEX Quote Layer
        ↓
PHASE 10 — Deterministic Risk/Policy Engine
        ↓
PHASE 11 — Execution Planner
        ↓
PHASE 12 — Real Cryptographic Signer
        ↓
PHASE 13 — Anchor DEX CPI Execution
        ↓
PHASE 14 — Actual Execution Measurement
        ↓
PHASE 15 — Purification
        ↓
PHASE 16 — Failure/Recovery/Safety
        ↓
PHASE 17 — Integration Tests
        ↓
PHASE 18 — Production Hardening
```

## Phase Descriptions & Key Deliverables

1. **PHASE 0 — Baseline & Safety**:
   - Workspace setup, dependency isolation, cargo checks, read-only guards, security invariants.
2. **PHASE 1 — Canonical Asset Registry**:
   - Pure domain models (`Asset`, `AssetIdentity`, `TokenDetails`, `ProviderConfig`), static asset identity.
3. **PHASE 2 — Asset ↔ Pyth Feed Mapping**:
   - Mapping canonical asset symbols and mints to exact Pyth price feed hex IDs and aliases.
4. **PHASE 3 — Dynamic Subscription Model**:
   - Subscription management for tracking and registering live oracle price feeds on-demand.
5. **PHASE 4 — Pyth Real-Time SSE Client**:
   - High-throughput Server-Sent Events (SSE) / WebSocket streaming client connecting to Hermes / Pyth Lazer.
6. **PHASE 5 — Dynamic Stream Manager**:
   - Lifecycle orchestration of feed connections, backpressure handling, reconnects, and feed additions/removals.
7. **PHASE 6 — Market Data Store**:
   - Low-latency in-memory cache and atomic state store for normalized real-time asset prices.
8. **PHASE 7 — Oracle Validation**:
   - Real-time price confidence intervals, staleness bounds (<=15s), exponential scaling, and outlier rejection.
9. **PHASE 8 — REST + WebSocket API**:
   - Actix/Axum endpoints and WebSocket pub/sub for client interactions and external telemetry.
10. **PHASE 9 — DEX Quote Layer**:
    - Integration with Jupiter v6 routing and Meteora Dynamic Bonding Curves (DBC) for swap price quotes.
11. **PHASE 10 — Deterministic Risk/Policy Engine**:
    - 4-defense lines: max position exposure, trade drift tolerance, minimum cash reserve floor, and stop-loss circuit breaker.
12. **PHASE 11 — Execution Planner**:
    - Atomic order synthesis, route selection, and slippage budget allocation decoupled from AI agent proposals.
13. **PHASE 12 — Real Cryptographic Signer**:
    - Backend-only hardware/env key isolation; zero signer credentials exposed to AI agents or frontend.
14. **PHASE 13 — Anchor DEX CPI Execution**:
    - Cross-Program Invocation (CPI) instruction construction for `equity_vault` program on Solana.
15. **PHASE 14 — Actual Execution Measurement**:
    - Post-settlement execution reconciliation: real slippage drift, confirmation duration, and fee accounting.
16. **PHASE 15 — Purification**:
    - Dividend and impure income calculation and charitable allocation logic adhering to Shariah standards.
17. **PHASE 16 — Failure/Recovery/Safety**:
    - Circuit breaker handling, RPC failover, dead-letter queuing, and emergency freeze mechanisms.
18. **PHASE 17 — Integration Tests**:
    - Multi-component integration test suite verifying the complete flow from oracle event to on-chain settlement.
19. **PHASE 18 — Production Hardening**:
    - Security audits, memory leak tests, high-concurrency benchmarks, zero-warning clippy/fmt verification.
