# Phase 06 — Execution

Status: DONE
Owner: Agent 6
Dependencies: Phase 05
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [x] verification
- [x] documentation
- [x] review

## Objective

Safely convert approved decisions into Solana transactions.

## Context

The Execution pipeline is the sole authorized bridge between off-chain decision intelligence and on-chain state mutations. It enforces strict pre-flight simulation, slippage protection, cryptographic key isolation, transaction confirmation, and auditable persistence into PostgreSQL.

## Flow

```
Decision
  ↓
Policy
  ↓
Risk
  ↓
Transaction Builder
  ↓
Simulation
  ↓
Signer
  ↓
Submit
  ↓
Confirm
  ↓
Persist
```

## Allowed Files

- `apps/api/src/engines/decision_engine/**`
- `apps/api/src/services/execution_service.rs`
- `apps/api/src/services/solana_service.rs`
- `apps/api/src/models/execution.rs`
- `apps/api/src/repositories/execution_repository.rs`
- `apps/api/src/routes/executions.rs`
- `tests/execution/**`

## Forbidden Files

- Direct access to private keys outside `signer.rs`
- Bypassing transaction simulation before signing
- Unbounded retry loops that risk double-execution

## Requirements

1. **Idempotency**: Every execution request must include a unique idempotency key (UUIDv4). Re-submitting the same key must never trigger a duplicate transaction.
2. **Pre-Flight Simulation**: All transactions must be simulated via Solana RPC (`simulateTransaction`) before signing. If simulation fails, abort immediately without signing.
3. **Slippage Limits**: Enforce strict slippage ceilings (max allowable price impact, e.g. $\le 150\text{ bps}$).
4. **Transaction Timeout**: Bound confirmation wait times (e.g. 30 seconds). Unconfirmed transactions transition to `EXPIRED` or trigger status checks rather than blind resubmission.
5. **Confirmation Handling**: Verify commitment status (`confirmed` or `finalized`).
6. **Bounded Retry Policy**: Maximum 3 retries with exponential backoff and blockhash refresh.
7. **Persistence**: Record all execution requests, status (`PENDING`, `SUBMITTED`, `CONFIRMED`, `FAILED`, `REJECTED`), signatures, and error codes in PostgreSQL `executions`.

## Implementation Steps

1. Implement `ExecutionService` in `apps/api/src/services/execution_service.rs`.
2. Integrate idempotency checks against PostgreSQL repository before building transactions.
3. Implement `TransactionBuilder` assembling instructions with priority fees (`ComputeBudgetInstruction`).
4. Execute RPC simulation check; verify zero error logs and compute units within limit.
5. Pass verified transaction to isolated `signer.rs` for cryptographic signature.
6. Submit via `SolanaService`, monitor confirmation status, and update execution record.
7. Expose REST route `GET /executions` and `GET /executions/:id`.

## Tests

- Rejected risk decision produces zero transaction.
- Simulation failure halts pipeline before signer is invoked.
- Duplicate idempotency key returns existing execution record without new transaction.
- Slippage exceedance triggers rejection.
- Timeout triggers clean error state without duplicate submission.

## Verification Commands

```bash
cargo test --package equity-catalyst-api execution
curl -s http://localhost:8080/executions | jq .
```

## Acceptance Criteria

- Any action rejected by the Risk Engine produces **zero** transaction attempts.
- Every on-chain transaction corresponds to a persisted PostgreSQL record with signature and status.
- Idempotency guarantees prevent duplicate executions.

## Documentation Updates

- Update `ai/invariants.md` if execution bounds are refined.
- Update `ai/current-state.md` upon completion.

## Failure Conditions

- Executing a transaction without prior simulation success.
- Signing or submitting an action that was rejected by the Risk Engine.
- Silent failure or duplicate submission on retry.
- Leaking private keys into logs or API responses.
