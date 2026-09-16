# Architecture Decision Records (ADRs)

This directory maintains the immutable architectural decisions guiding Equity Catalyst's design and implementation.

---

## Index of Decisions

- [ADR-001: Rust & Axum Backend Service](file:///home/seam/Desktop/project/equity-catalyst/ai/decisions/ADR-001-rust-api.md) — Selection of Rust, Axum, and Tokio for off-chain safety, predictable latency, and native Solana SDK integration.
- [ADR-002: Pyth Pro & Hermes Integration](file:///home/seam/Desktop/project/equity-catalyst/ai/decisions/ADR-002-pyth-pro.md) — Utilizing Pyth real-time equity oracle feeds with confidence intervals for mark-to-market valuations.
- [ADR-003: Meteora DBC with 3-Regime Curve](file:///home/seam/Desktop/project/equity-catalyst/ai/decisions/ADR-003-meteora-dbc.md) — Tailoring Meteora Dynamic Bonding Curves to institutional equity price discovery and DAMM v2 graduation.
- [ADR-004: Provider Adapter Pattern](file:///home/seam/Desktop/project/equity-catalyst/ai/decisions/ADR-004-provider-adapters.md) — Normalizing external protocols (Pyth, Jupiter, PreStocks, Tessera) into decoupled domain adapters.
- [ADR-005: Agent Safety & Dual-Line Risk Engine](file:///home/seam/Desktop/project/equity-catalyst/ai/decisions/ADR-005-agent-safety.md) — Strict separation between probabilistic LLM agent synthesis and deterministic risk/signer enforcement.
- [ADR-006: Mainnet-First Canonical Assets](file:///home/seam/Desktop/project/equity-catalyst/ai/decisions/ADR-006-mainnet-first.md) — Strict adherence to real mainnet tokenized assets (Backed Finance xStocks) rather than synthetic or mock tokens.
