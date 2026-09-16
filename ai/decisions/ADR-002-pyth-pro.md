# ADR-002: Real-Time Equity Oracles with Pyth Pro & Hermes

## Status
Accepted

## Context
Valuing tokenized equities and collateralized vaults requires accurate, institutional-grade market prices from traditional equity exchanges (NASDAQ, NYSE). Conventional decentralized oracles often suffer from minutes-long latency or lack coverage for non-crypto assets.

## Decision
We integrate **Pyth Network** via its Hermes low-latency REST and streaming API:
- Ingest real-time price feeds for target equities (`NVDA`, `AAPL`, `SPY`).
- Price normalization pipeline parses price mantissa and exponent ($price \cdot 10^{expo}$).
- Enforce confidence interval bounds: if $\frac{\text{confidence}}{\text{price}} > 0.02$ (2%), the price is considered untrusted and rejected for mark-to-market calculations.
- Enforce freshness ceiling: reject quotes older than 60 seconds.

## Consequences
### Positive
- Sub-second equity price feeds backed by primary market makers and institutional exchanges.
- Transparent confidence intervals allow the Risk Engine to prevent execution during erratic market openings or halts.

### Negative
- Dependence on Pyth Hermes infrastructure availability; requires fallback circuit breakers when feeds are stale.
