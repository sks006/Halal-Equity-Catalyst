# Security & Risk Review (Phase 09 Hardened)

Status: COMPLETED (Phase 09 Security Hardening Verified)
Review Date: 2026-09-18
Scope: Whole Workspace (`equity-catalyst-shared`, `equity-catalyst-api`, `equity-catalyst-solana`, `equity-catalyst-pyth`, `equity-catalyst-jupiter`, `equity-vault`)

---

## 1. Threat Model & Invariant Protection Matrix

| Threat / Attack Vector | Severity | System Defense Layer | Implemented Mechanism | Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| **Agent Prompt Injection** | Critical | Decision Engine & Risk Engine | Agent only outputs advisory proposals; deterministic pipeline enforces policy, Shariah gate, and spot ownership | `test_prompt_injection_containment_in_reasoning` |
| **Key Leakage in Logs/Errors** | Critical | Signer & Error Pipeline | `RedactedSecret<T>` wrapper masks `Display`/`Debug` as `[REDACTED]`; `ExecutionSigner` custom `Debug` masks key material; `ApiError` scrubs connection strings and passwords | `test_secret_redaction_and_error_sanitization` |
| **Unauthorized State Mutation** | High | API Gateway Middleware | `require_admin_auth` enforces constant-time API key verification (`x-admin-key` or `Bearer`), rejecting missing or invalid keys with HTTP 401 | `test_admin_auth_missing_credential_rejected_401`, `test_admin_auth_invalid_credential_rejected_401` |
| **DDoS / Resource Exhaustion** | High | API Gateway Middleware | Sliding-window `RateLimiter` with bounded per-client tracking and periodic pruning; rejects burst overages with HTTP 429 | `test_rate_limiting_burst_rejection_429_and_recovery` |
| **Replay / Duplicate Execution** | High | Execution Service & DB | Strict idempotency on UUID across all states (`requested`, `simulated`, `submitted`, `confirmed`) with database unique constraint collision handling | `test_idempotency_sequential_and_concurrent_duplicate_handling` |
| **Execution on Inactive/Paused Vault** | High | Execution Boundary | Multi-checkpoint emergency pause guard: rechecks vault pause and policy active status immediately before Step 5 (signing boundary); signing never occurs | `test_signer_spy_never_invoked_on_validation_or_risk_rejection` |
| **Poison Pill / Worker Crash** | Medium | Background Supervisors | `WorkerSupervisor` catches worker task panics via `tokio::spawn`, applies bounded exponential backoff, and tracks restart windows; unprocessable events recorded to `dead_letters` table | `test_dead_letter_records_unprocessable_events`, `test_supervisor_restarts_crashing_worker_up_to_limit` |
| **RPC 429/503 Cascading Failure** | Medium | Solana Integration RPC | Differentiates permanent client errors (400, 401, 403, 404, fail closed immediately) from transient network errors (429, 502, 503, 504, retried with jitter and backoff) | `test_rpc_client_failover_on_429`, `test_rpc_client_failover_on_503` |
| **Integer Overflow & Drift** | High | Shared Math Core | 128-bit intermediate arithmetic for basis points, half-up rounding, explicit monotonic `ThresholdComparison` (< vs <=) | `test_property_basis_points_scale_and_overflow_safety`, `test_property_deterministic_fee_breakdown_invariant`, `test_property_shariah_threshold_monotonicity` |

---

## 2. Hardening Invariant Verification

The system enforces 10 immutable security rules:
1. **AI agents never receive private keys**: Keys reside exclusively in `ExecutionSigner` off-chain and never pass to AI prompts or API responses.
2. **AI agents never sign transactions**: Signing is strictly executed in `ExecutionEngineService::execute_transaction` after Step 4 simulation passes.
3. **AI agents never modify Shariah compliance state**: Shariah status is governed by deterministic on-chain compliance PDAs and off-chain `ShariahAssetRegistry`.
4. **Rejected Shariah assets can never reach execution**: Shariah gate revalidates asset status prior to building transaction.
5. **Expired compliance can never reach execution**: Review expiration timestamp is checked against block/wall-clock time before transaction construction.
6. **Unowned SELL operations can never reach execution**: `validate_spot_ownership` verifies vault position balance $\ge$ requested sell quantity.
7. **Unfunded BUY operations can never reach execution**: `validate_spot_funding` verifies settled cash balance $\ge$ gross purchase requirement.
8. **Prohibited financial instruments**: Margin, leverage, shorting, lending, and borrowing are strictly rejected by `RiskEngine`.
9. **Idempotency**: Execution UUIDs prevent double execution across network retries or concurrent submissions.
10. **Pre-signing simulation and pause gate**: Every transaction is simulated before signing; any pause triggered between simulation and signing immediately aborts the transaction.

---

## 3. Dependency & Tooling Audit

- `cargo clippy --workspace -- -D warnings`: Clean pass with 0 warnings.
- `cargo fmt --all -- --check`: Clean pass with 0 formatting discrepancies.
- `cargo audit`: **NOT AVAILABLE** (cargo-audit binary is not installed on this host environment).
- `pnpm audit`: Identified 6 vulnerabilities in transitive dev/client dependencies (2 moderate, 4 high in `ws`/`uuid`/`serialize-javascript` via `@solana/web3.js` and `mocha`). Transitive updates should be coordinated when upstream Solana web3.js publishes updated dependency trees.

---

## 4. Operational Assessment & Remaining Production Requirements

- **Factual Posture**: Implemented and tested against documented adversarial failure scenarios, network dropouts, rate-limiting overages, and concurrent execution collisions.
- **Production Prerequisites**:
  1. Real mainnet Meteora DBC pool deployment (requires operational wallet SOL funding).
  2. Multi-signature authority for the on-chain Anchor vault and compliance update authority.
  3. Production HSM or AWS KMS / GCP Cloud KMS integration for `ExecutionSigner` key storage in place of local file keypairs.
