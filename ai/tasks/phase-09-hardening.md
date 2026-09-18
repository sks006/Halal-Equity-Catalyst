# Phase 09 — Hardening

Status: READY_FOR_REVIEW
Owner: unassigned
Dependencies: Phase 06, Phase 08
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [x] verification
- [x] documentation
- [x] review

## Objective

Harden security, operational resilience, and automated error recovery across all system layers.

## Context

Prior to production readiness, the entire infrastructure must undergo rigorous hardening: secret scrubbing, signer isolation, API rate-limiting, replay protection, circuit breakers, automated background worker recovery, and dead-letter queues.

## Allowed Files

- `apps/api/src/config.rs`
- `apps/api/src/router.rs`
- `apps/api/src/error.rs`
- `apps/api/src/workers/**`
- `apps/api/src/engines/risk_engine/**`
- `apps/api/src/services/**`
- `crates/shared/**`
- `tests/**`

## Forbidden Files

- Disabling security checks or clippy warnings via permissive compiler flags
- Storing unencrypted keys or fallback secrets in source code

## Requirements

1. **Security Hardening**:
   - **Secrets**: Ensure zero secret leakage in logs, headers, or client bundles.
   - **Signing**: Isolate transaction signing to dedicated signing modules with access auditing.
   - **API Authentication**: Protect administrative routes with API key / bearer token validation.
   - **Rate Limits**: Implement IP and account rate limiting on public endpoints (`tower-http`).
   - **Validation**: Enforce boundary input validation on all DTOs.
   - **Replay Protection**: Enforce UUID idempotency keys on every state-mutating execution.
   - **Emergency Stop**: Provide global and per-vault emergency pause instructions halting all non-exit operations.
2. **Reliability & Resilience**:
   - **Pyth Reconnect**: Implement exponential backoff and heartbeat recovery on WebSocket streaming drops.
   - **RPC Retry**: Implement jittered exponential retries on transient Solana RPC 429 / 503 errors.
   - **Database Retry**: Resilient connection recovery on PostgreSQL connection pool hiccups.
   - **Worker Restart**: Background worker supervision restarting crashed listener or worker tasks.
   - **Dead-Letter Handling**: Queue unparseable or rejected events into a dead-letter log for postmortem analysis.

## Implementation Steps

1. Configure `tower-http` rate-limiting and request tracing middleware in `apps/api/src/router.rs`.
2. Add dead-letter queue table/channel in Redis and PostgreSQL.
3. Enhance `EventListener` and `PolicyWorker` with supervisor loops that auto-restart on unhandled panic or network drop.
4. Audit all `tracing::info!` and `tracing::error!` statements to guarantee zero sensitive data logging.
5. Run full workspace test suite and enforce zero clippy warnings.

## Tests

- Rate-limiter threshold rejection test (HTTP 429 Too Many Requests).
- Replay protection test: second request with identical idempotency key is safely deduplicated.
- Emergency stop test: paused vault rejects deposits, borrows, and trades.
- RPC retry test simulating 3 transient network failures followed by successful confirmation.
- Pyth WebSocket auto-reconnect test upon socket severance.

## Verification Commands

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
pnpm lint
pnpm build
```

## Acceptance Criteria

- `cargo check --workspace` and `cargo test --workspace` pass with zero failures.
- `cargo clippy --workspace -- -D warnings` completes with zero warnings.
- Frontend linter and production build (`pnpm lint`, `pnpm build`) pass cleanly.
- Emergency stop halts all trading operations immediately.

## Documentation Updates

- Document incident procedures and runbooks in `ai/debug/runbook.md`.
- Update `ai/reviews/security-review.md` and `ai/current-state.md`.

## Failure Conditions

- Any compiler or clippy warning remaining under `-D warnings`.
- Unhandled panic causing complete server termination without supervisor recovery.
- Sensitive credentials present in application logs or git history.
