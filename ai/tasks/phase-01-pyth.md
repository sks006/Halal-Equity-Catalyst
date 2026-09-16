# Phase 01 — Pyth Pro

Status: DONE
Owner: Agent 1
Dependencies: Phase 00
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [x] verification
- [x] documentation
- [x] review

## Objective

Integrate Pyth Pro as the authoritative market-data layer.

## Context

Establish institutional-grade real-time price feeds for traditional equities and tokenized assets using Pyth Pro / Hermes. This service powers mark-to-market valuations, premium/discount tracking between off-chain stocks and on-chain RWAs, and circuit breakers for automated execution.

## Allowed Files

- `apps/api/src/providers/pyth/**`
- `apps/api/src/services/pyth_service.rs`
- `apps/api/src/workers/pyth_stream.rs`
- `apps/api/src/routes/oracle.rs`
- `apps/api/src/routes/markets.rs`
- `integrations/pyth/**`
- `crates/shared/src/models/**`
- `tests/pyth/**`

## Forbidden Files

- `apps/web/**` (PRO_API_KEY must never be exposed to frontend code)
- `programs/equity_vault/**` (programs do not make HTTP calls)
- Direct commit of `.env` files containing raw API keys

## Requirements

1. **REST Client**: Hermes endpoint integration (`/v2/updates/price/latest`).
2. **History Client**: Benchmark TWAP and historical price updates.
3. **WebSocket Client**: Real-time streaming feed with auto-reconnection and exponential backoff.
4. **Feed Registry**: Verified lookup for canonical equity and tokenized pairs:
   - `Equity.US.AAPL/USD`
   - `Crypto.AAPLX/USD`
   - `Crypto.AAPLON/USD`
   - *Rule: Never hardcode unverified feed IDs.*
5. **Freshness Calculation**: Reject updates older than 60 seconds as stale.
6. **Normalized MarketPrice**: Strict representation converting mantissa/exponent ($price \cdot 10^{expo}$) and confidence intervals.
7. **Cross-Market Comparison**: Compute premium/discount basis points between traditional equity reference and tokenized RWA.
8. **Security Standards**:
   - Never expose `PRO_API_KEY` to client or frontend.
   - Use `Authorization: Bearer <PRO_API_KEY>`.
   - Typed response models and error payloads.
   - Bounded request timeouts (5s) and bounded retries (max 3).

## Implementation Steps

1. Configure Pyth Pro HTTP client using `reqwest` with Bearer auth headers and timeout configuration.
2. Build feed discovery/resolution service verifying live Hermes feed IDs.
3. Implement `MarketPrice` normalization logic with confidence bound gating ($\frac{\text{conf}}{\text{price}} \le 2.0\%$).
4. Implement WebSocket streaming worker in `apps/api/src/workers/pyth_stream.rs` with heartbeat and reconnection logic.
5. Build cross-market comparison engine calculating `premium_bps = (token_price - ref_price) * 10_000 / ref_price`.
6. Implement `GET /markets/:symbol` endpoint exposing normalized underlying, tokenized price, premium, and health status.
7. Unit and integration test all failure and edge-case paths.

## Tests

- Valid response parsing and exponent normalization.
- Unauthorized (401/403) status handling.
- HTTP timeout handling.
- Malformed JSON payload handling.
- Stale timestamp detection and rejection.
- Duplicate price update deduplication.
- WebSocket disconnect and reconnect recovery.

## Verification Commands

```bash
cargo test -p equity-catalyst-pyth
cargo test --package equity-catalyst-api pyth
curl -s http://localhost:8080/markets/AAPL | jq .
curl -s http://localhost:8080/oracle/price/AAPL | jq .
```

## Acceptance Criteria

- Normalized Pyth prices and confidence metrics appear in `GET /markets/:symbol`.
- Stale feeds (> 60s) or feeds exceeding confidence bounds trigger `HEALTH_CRITICAL` and fail closed.
- `PRO_API_KEY` is strictly confined to server-side memory.

## Documentation Updates

- Record verified live feed IDs in `ai/context/pyth.md`.
- Update `ai/current-state.md` with Pyth Pro phase status.

## Failure Conditions

- `PRO_API_KEY` appears in any frontend bundle, git diff, or public API response.
- Hardcoded unverified feed IDs in production code.
- Allowing stale or wide-confidence prices to authorize automated execution.
- Unbounded retry loops or unbounded queue allocations on socket disconnects.
