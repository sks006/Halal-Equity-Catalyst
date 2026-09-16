# Phase 04 — Equity Engine

Status: DONE
Owner: Agent 4
Dependencies: Phase 01, Phase 03
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [x] verification
- [x] documentation
- [x] review

## Objective

Turn Pyth market data + DBC state into equity-specific market intelligence.

## Context

Tokenized stocks and pre-IPO assets fluctuate relative to their underlying traditional exchange benchmarks (NASDAQ/NYSE via Pyth Pro) and dynamic on-chain bonding curve reserves (Meteora DBC). The Equity Engine processes these dual feeds to calculate institutional metrics—premium, discount, spread, volatility, and curve progress—and synthesizes an authoritative `EquityHealth` evaluation.

## Allowed Files

- `apps/api/src/engines/market_engine/**`
- `apps/api/src/engines/equity_engine.rs`
- `apps/api/src/services/market_service.rs`
- `apps/api/src/routes/markets.rs`
- `crates/shared/src/models/market.rs`
- `crates/shared/src/math/equity.rs`
- `tests/equity_engine/**`

## Forbidden Files

- Direct transaction signing logic
- Smart contract execution code
- External HTTP clients inside `crates/shared`

## Requirements

1. **Metrics Calculation**:
   - `reference_price`: Institutional fair-value price from Pyth Pro.
   - `token_price`: Real-time spot price derived from Meteora DBC sqrtPrice.
   - `premium_bps`: Basis points difference when $P_{token} > P_{ref}$.
   - `discount_bps`: Basis points difference when $P_{token} < P_{ref}$.
   - `spread_bps`: Bid/ask and market deviation spread.
   - `volatility`: Realized price variance over moving window.
   - `freshness`: Elapsed seconds since latest oracle update.
   - `liquidity`: Total base and quote reserves committed in curve.
   - `curve_progress`: Percentage of curve threshold achieved toward graduation.
2. **Authoritative Output**:
   - `EquityHealth` enum:
     - `HEALTHY`: Premium within standard band ($\le \pm 200\text{ bps}$), oracle fresh ($\le 30\text{s}$), low volatility.
     - `WARNING`: Elevated premium/discount ($\pm 200\text{ to } \pm 500\text{ bps}$), oracle delay ($30\text{s to } 60\text{s}$), or sharp volume spike.
     - `CRITICAL`: Severe decoupling ($> \pm 500\text{ bps}$), stale oracle ($> 60\text{s}$), confidence interval breach ($> 2.0\%$), or zero denominator.

## Implementation Steps

1. Implement deterministic financial math in `crates/shared/src/math/equity.rs`:
   ```rust
   premium_bps = (token_price - ref_price) * 10_000 / ref_price
   ```
2. Build `MarketEngine` in `apps/api/src/engines/market_engine/` aggregating Pyth oracle cache and Meteora pool state.
3. Add denominator guard: return explicit `DivisionByZero` or `InvalidReferencePrice` error if `ref_price <= 0`.
4. Implement `EquityHealth::evaluate` state machine mapping input metrics to health states.
5. Expose comprehensive metrics via `GET /markets/:symbol/intelligence`.
6. Write unit tests covering exact metric scenarios.

## Tests

- Reference = 100, Token = 102 $\implies$ Premium = `+200 bps` (`HEALTHY`).
- Reference = 100, Token = 98 $\implies$ Discount / Premium = `-200 bps` (`HEALTHY`).
- Reference = 100, Token = 110 $\implies$ Premium = `+1000 bps` (`CRITICAL`).
- Reference = 0 $\implies$ returns explicit Error (`InvalidReferencePrice`).
- Oracle staleness = 75s $\implies$ triggers `CRITICAL`.
- Confidence bound = 3% $\implies$ triggers `CRITICAL`.

## Verification Commands

```bash
cargo test -p equity-catalyst-shared math::equity
cargo test --package equity-catalyst-api market_engine
curl -s http://localhost:8080/markets/NVDAx/intelligence | jq .
```

## Acceptance Criteria

- All specified test calculations pass exactly without floating-point drift.
- Reference price equal to zero safely returns an error without panicking.
- `EquityHealth` outputs correctly evaluate across `HEALTHY`, `WARNING`, and `CRITICAL` states.

## Documentation Updates

- Update `ai/domain.md` if additional metrics are formalized.
- Update `ai/current-state.md` upon completion.

## Failure Conditions

- Panicking on division by zero when reference price is 0 or negative.
- Using imprecise floating point calculations for authoritative risk comparisons.
- Emitting `HEALTHY` when oracle price is stale or confidence bounds are violated.
