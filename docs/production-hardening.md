# Production Hardening Review

This document provides a comprehensive security and production readiness audit for Phase 18 of the **Halal-Equity-Catalyst** project. Every item reviews critical production infrastructure across on-chain Anchor programs, off-chain Rust microservices, database schemas, risk engines, and external oracle/DEX integrations.

---

## Hardening Matrix (30 Review Items)

| # | Domain / Review Item | Severity | Affected Component | Status |
|---|----------------------|----------|-------------------|--------|
| 1 | Secrets Management | High | `apps/api/src/config.rs` | Hardened / Verified |
| 2 | Private-Key Exposure | Critical | `crates/shared`, `apps/api/src/engines/decision_engine/signer` | Hardened / Verified |
| 3 | Logging of Sensitive Data | Medium | `apps/api/src/main.rs`, `apps/api/src/config.rs` | Hardened / Verified |
| 4 | Database Constraints | High | `db/migrations/012_production_hardening_constraints.sql` | Hardened / Verified |
| 5 | Transaction Idempotency | Critical | `apps/api/src/engines/execution_planner/idempotency.rs` | Hardened / Verified |
| 6 | Retry Behavior | Medium | `integrations/solana/src/client.rs`, `integrations/pyth/src/stream.rs` | Hardened / Verified |
| 7 | Timeouts | Medium | `integrations/solana`, `integrations/jupiter`, `integrations/pyth` | Hardened / Verified |
| 8 | HTTP Connection Limits | Medium | `apps/api/src/router.rs`, `apps/api/src/lib.rs` | Hardened / Verified |
| 9 | WebSocket Backpressure | Medium | `apps/api/src/routes/market_data.rs` | Hardened / Verified |
| 10 | Pyth Reconnect Behavior | High | `integrations/pyth/src/stream.rs` | Hardened / Verified |
| 11 | DEX Quote Expiry | High | `apps/api/src/services/quote_service.rs`, `crates/shared/src/types/market_quote.rs` | Hardened / Verified |
| 12 | Oracle Freshness | Critical | `apps/api/src/services/market_data_store.rs`, `crates/shared/src/types/market_data.rs` | Hardened / Verified |
| 13 | Oracle Confidence | High | `apps/api/src/services/market_data_store.rs`, `apps/api/src/services/oracle_service.rs` | Hardened / Verified |
| 14 | Solana Transaction Confirmation | High | `integrations/solana/src/client.rs`, `apps/api/src/engines/decision_engine/submitter.rs` | Hardened / Verified |
| 15 | Replay Protection | Critical | `programs/equity_vault/src/instructions/execute_action.rs`, `apps/api/src/engines/execution_planner` | Hardened / Verified |
| 16 | Access Control | High | `apps/api/src/middleware/auth.rs`, `programs/equity_vault/src/instructions` | Hardened / Verified |
| 17 | Rate Limiting | Medium | `apps/api/src/middleware/rate_limit.rs` | Hardened / Verified |
| 18 | Observability | Medium | `apps/api/src/routes/health.rs`, `apps/api/src/routes/metrics.rs` | Hardened / Verified |
| 19 | Metrics | Medium | `apps/api/src/routes/metrics.rs` | Hardened / Verified |
| 20 | Structured Logs | Low | `apps/api/src/main.rs`, `apps/api/src/services` | Hardened / Verified |
| 21 | Error Classification | High | `apps/api/src/error.rs`, `apps/api/src/engines/pipeline_safety.rs` | Hardened / Verified |
| 22 | Graceful Shutdown | Medium | `apps/api/src/main.rs` | Hardened / Verified |
| 23 | Configuration Validation | High | `apps/api/src/config.rs` | Hardened / Verified |
| 24 | Dev Secrets in Production | Critical | `apps/api/src/config.rs` | Hardened / Verified |
| 25 | Hardcoded Feed IDs | High | `apps/api/src/services/asset_watcher.rs`, `db/migrations/011_asset_market_data.sql` | Hardened / Verified |
| 26 | Hardcoded Private Keys | Critical | `apps/api/src/engines/decision_engine/signer` | Hardened / Verified |
| 27 | `f64` in Financial / Execution Math | High | `apps/api/src/engines/risk_engine/mod.rs`, `apps/api/src/services/market_data_store.rs` | Hardened / Verified |
| 28 | Symbol-Based Asset Identity | Critical | `crates/shared/src/models/asset.rs`, `apps/api/src/services/market_data_store.rs` | Hardened / Verified |
| 29 | Fail-Open Shariah Screening | Critical | `programs/equity_vault/src/instructions/execute_action.rs`, `apps/api/src/engines/risk_engine` | Hardened / Verified |
| 30 | Arbitrary CPI Program Invocation | Critical | `programs/equity_vault/src/dex/mod.rs`, `programs/equity_vault/src/instructions/execute_action.rs` | Hardened / Verified |

---

## Detailed Findings, Remediation & Verification

### 1. Secrets Management
* **Finding:** Application configuration previously lacked strict environment validation on startup, risking unconfigured database connections or default credentials in live environments.
* **Severity:** High
* **Affected Component:** [`apps/api/src/config.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/config.rs)
* **Evidence:** In `Config::from_env()`, environment variables fall back to default structs unless strictly evaluated against target environment boundaries.
* **Remediation:** Added `Config::validate()` method invoked during server startup in `main.rs`. In `Environment::Mainnet`, `database_url` cannot be empty, and administrative keys must meet high entropy requirements ($\ge 16$ characters).
* **Verification:** [`test_config_validation_rejects_dev_secret_in_mainnet`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/production_hardening_test.rs#L36-L61) verifies startup rejection when configuration is invalid.

### 2. Private-Key Exposure
* **Finding:** Risk of private keys leaking to loggers, serialized responses, or planners in downstream modules.
* **Severity:** Critical
* **Affected Component:** [`apps/api/src/engines/decision_engine/signer/mod.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/decision_engine/signer/mod.rs), [`crates/shared`](file:///home/seam/Desktop/project/equity-catalyst/crates/shared)
* **Evidence:** Debug trait derivation on signer types or plan structures could print key bytes if not manually masked.
* **Remediation:** `TransactionBuilder` and `ExecutionPlanner` operate strictly on unsigned transaction payloads and public keys (`Pubkey`). `DevTestSigner` and `KeypairSigner` implement custom `Debug` displaying `"[REDACTED KEYPAIR]"`.
* **Verification:** Unit tests in [`signer.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/decision_engine/signer/mod.rs) confirm `format!("{:?}", signer)` contains `[REDACTED]` and zero raw key bytes.

### 3. Logging of Sensitive Data
* **Finding:** Connection URIs for PostgreSQL and Redis printed during service bootstrap may contain plaintext usernames and passwords.
* **Severity:** Medium
* **Affected Component:** [`apps/api/src/config.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/config.rs), [`apps/api/src/main.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/main.rs)
* **Evidence:** Startup log lines previously printed raw `config.redis_url`.
* **Remediation:** Implemented `sanitize_connection_url(raw: &str) -> String` which strips and masks password credentials (e.g. `redis://:***@host:port`), ensuring sanitized logging.
* **Verification:** [`test_sanitize_connection_url`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/production_hardening_test.rs#L92-L105) asserts passwords in Postgres and Redis connection strings are replaced with `***`.

### 4. Database Constraints
* **Finding:** `executions` table lacked `CHECK` constraints ensuring positive `amount_in` and valid `slippage_bps`, and lacked partial unique constraints preventing duplicate on-chain execution rows.
* **Severity:** High
* **Affected Component:** [`db/migrations/012_production_hardening_constraints.sql`](file:///home/seam/Desktop/project/equity-catalyst/db/migrations/012_production_hardening_constraints.sql)
* **Evidence:** In `005_executions.sql`, columns `amount_in` and `slippage_bps` lacked `CHECK (amount_in > 0)` and `CHECK (slippage_bps BETWEEN 0 AND 10000)`.
* **Remediation:** Created migration `012_production_hardening_constraints.sql` adding `chk_executions_positive_amount_in`, `chk_executions_valid_slippage`, unique index on `tx_signature WHERE tx_signature IS NOT NULL`, and partial unique index on `(policy_decision_id) WHERE status IN ('CONFIRMED', 'PENDING')`.
* **Verification:** Verified SQL schema syntax and execution constraints in database migration pipeline.

### 5. Transaction Idempotency
* **Finding:** `IdempotencyTracker` stored execution plans indefinitely in an in-memory hash map, which in high-volume production would cause unbounded heap growth and memory exhaustion.
* **Severity:** Critical
* **Affected Component:** [`apps/api/src/engines/execution_planner/idempotency.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/execution_planner/idempotency.rs)
* **Evidence:** `IdempotencyTracker::check_and_register` inserted entries but never removed expired ones.
* **Remediation:** Added `pub async fn prune_expired(&self, current_time: i64) -> usize` to prune expired idempotency records and their secondary decision indexes while preserving active records.
* **Verification:** [`test_idempotency_tracker_prune_expired`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/production_hardening_test.rs#L107-L167) verifies cleanup of expired entries and retention of unexpired plans.

### 6. Retry Behavior
* **Finding:** Direct Solana RPC transaction submission can fail intermittently during network congestion if submitted without exponential backoff and jitter.
* **Severity:** Medium
* **Affected Component:** [`integrations/solana/src/client.rs`](file:///home/seam/Desktop/project/equity-catalyst/integrations/solana/src/client.rs)
* **Evidence:** `SolanaClient` handles send and confirm operations across primary and fallback RPC endpoints.
* **Remediation:** Hardened `SolanaClient::send_and_confirm_transaction` with retry intervals, fallback RPC rotation, and bounded retry counts.
* **Verification:** Validated by [`test_failure_case_failed_transaction_submission_error`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/end_to_end_safety_pipeline_test.rs#L254).

### 7. Timeouts
* **Finding:** External HTTP and WebSocket connections (Hermes, Jupiter, Solana RPC) without strict timeouts could hang worker threads indefinitely.
* **Severity:** Medium
* **Affected Component:** [`apps/api/src/config.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/config.rs), [`integrations/solana`](file:///home/seam/Desktop/project/equity-catalyst/integrations/solana)
* **Evidence:** Solana RPC defaults configured via `solana_rpc_timeout_ms` (15,000ms), Hermes HTTP client sets explicit 10s timeout.
* **Remediation:** Enforced configurable timeouts across all network clients; reject zero or non-sensical timeouts during configuration validation.
* **Verification:** Confirmed timeout configurations across `JupiterClient`, `PythClient`, and `SolanaService`.

### 8. HTTP Connection Limits & Body Limits
* **Finding:** Axum server lacked explicit body size limit on public endpoints, leaving routes vulnerable to denial-of-service via large request payloads.
* **Severity:** Medium
* **Affected Component:** [`apps/api/src/router.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/router.rs)
* **Evidence:** `apps/api/src/router.rs` did not configure `DefaultBodyLimit`.
* **Remediation:** Injected `DefaultBodyLimit::max(1024 * 1024)` (1MB) as global middleware in `create_router()`.
* **Verification:** [`test_request_body_size_limit`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/production_hardening_test.rs#L199-L215) confirms 1.5MB payloads are rejected with `413 Payload Too Large`.

### 9. WebSocket Backpressure
* **Finding:** Slow WebSocket consumers could cause unbounded channel buffer growth in the broadcast price stream.
* **Severity:** Medium
* **Affected Component:** [`apps/api/src/routes/market_data.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/routes/market_data.rs), [`apps/api/src/services/market_data_store.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/services/market_data_store.rs)
* **Evidence:** Bounded channels `tokio::sync::broadcast::channel(1024)` and WebSocket sender channel `tokio::sync::mpsc::channel(64)`.
* **Remediation:** Slow receivers that cannot keep up encounter lag errors and are dropped rather than exhausting server memory.
* **Verification:** Verified bounded channel capacities in `market_data_store.rs` and `market_data_ws_handler`.

### 10. Pyth Reconnect Behavior
* **Finding:** If Pyth Hermes SSE stream disconnects, pricing would stop updating without proactive reconnect or marking data stale.
* **Severity:** High
* **Affected Component:** [`integrations/pyth/src/stream.rs`](file:///home/seam/Desktop/project/equity-catalyst/integrations/pyth/src/stream.rs)
* **Evidence:** `DynamicStreamManager` handles SSE connection lifecycle.
* **Remediation:** Integrated exponential backoff reconnection loop with jitter, tracking reconnect counts and automatically flagging feeds as stale when disconnected beyond tolerance.
* **Verification:** Tested in [`apps/api/tests/market_data_store_test.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/market_data_store_test.rs) and Phase 16/17 pipeline safety tests.

### 11. DEX Quote Expiry
* **Finding:** Market conditions drift rapidly; an unexpired DEX quote must never be executed past its validity window.
* **Severity:** High
* **Affected Component:** [`apps/api/src/services/quote_service.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/services/quote_service.rs), [`crates/shared/src/types/market_quote.rs`](file:///home/seam/Desktop/project/equity-catalyst/crates/shared/src/types/market_quote.rs)
* **Evidence:** `MarketQuote::is_expired(current_time)` checks `valid_until <= current_time`.
* **Remediation:** Deterministic policy authorizer and execution planner re-validate quote expiration; expired quotes fail closed with `PolicyRejectionReason::QuoteExpired` and `PlannerError::QuoteExpired`.
* **Verification:** [`test_failure_case_expired_quote`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/end_to_end_safety_pipeline_test.rs#L210) verifies execution is blocked when quote expires.

### 12. Oracle Freshness
* **Finding:** Oracle prices can become stale during upstream Pyth outages or Solana network stalls.
* **Severity:** Critical
* **Affected Component:** [`apps/api/src/services/market_data_store.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/services/market_data_store.rs), [`crates/shared/src/types/market_data.rs`](file:///home/seam/Desktop/project/equity-catalyst/crates/shared/src/types/market_data.rs)
* **Evidence:** `MarketPriceUpdate::is_fresh(max_staleness_secs)` evaluates `|now - publish_time| <= max_staleness`.
* **Remediation:** Risk authorizer enforces maximum price age (default 30s); stale prices trigger immediate rejection with `PolicyRejectionReason::StaleOraclePrice`.
* **Verification:** [`test_failure_case_stale_oracle`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/end_to_end_safety_pipeline_test.rs#L182) verifies fail-closed rejection.

### 13. Oracle Confidence
* **Finding:** During illiquid or volatile periods, Pyth confidence intervals expand, indicating uncertain market valuation.
* **Severity:** High
* **Affected Component:** [`apps/api/src/services/market_data_store.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/services/market_data_store.rs), [`apps/api/src/engines/decision_engine/policy.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/decision_engine/policy.rs)
* **Evidence:** `conf_bps` is calculated as `(conf_scaled * 10,000) / price_scaled`.
* **Remediation:** Confidence ratio checked against policy maximum (e.g. 200 bps = 2%); excess confidence triggers `PolicyRejectionReason::ConfidenceTooLarge`.
* **Verification:** Tested in [`test_safety_guard_pyth_checks`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/pipeline_safety.rs#L341).

### 14. Solana Transaction Confirmation
* **Finding:** Transaction submission without checking commitment level can cause the application to assume execution succeeded before it is finalized or when dropped by the mempool.
* **Severity:** High
* **Affected Component:** [`integrations/solana/src/client.rs`](file:///home/seam/Desktop/project/equity-catalyst/integrations/solana/src/client.rs), [`apps/api/src/engines/decision_engine/submitter.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/decision_engine/submitter.rs)
* **Evidence:** Transaction submitter awaits confirmation with commitment level `Confirmed`.
* **Remediation:** Submission checks signature status across multiple RPC retries with configurable timeout. Timeouts return structured `TransactionConfirmationTimeout` error without double-submitting.
* **Verification:** [`test_failure_case_failed_transaction_submission_error`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/end_to_end_safety_pipeline_test.rs#L254).

### 15. Replay Protection
* **Finding:** Malicious re-dispatch of a previously signed trade transaction or re-use of an approved policy decision.
* **Severity:** Critical
* **Affected Component:** [`programs/equity_vault/src/instructions/execute_action.rs`](file:///home/seam/Desktop/project/equity-catalyst/programs/equity_vault/src/instructions/execute_action.rs), [`apps/api/src/engines/execution_planner/idempotency.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/execution_planner/idempotency.rs)
* **Evidence:** On-chain instruction initializes PDA with `seeds = [EXECUTION_SEED, vault.key().as_ref(), &execution_id.to_le_bytes()]`.
* **Remediation:** Dual-layer defense: Anchor prevents on-chain re-execution via PDA account initialization failure (`AccountAlreadyInitialized`); off-chain `IdempotencyTracker` enforces unique `idempotency_key` and canonical digest checks.
* **Verification:** [`test_replay_idempotency_pda_seeds`](file:///home/seam/Desktop/project/equity-catalyst/programs/equity_vault/tests/dex_cpi_execution_test.rs#L137) and [`test_failure_case_duplicate_execution`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/end_to_end_safety_pipeline_test.rs#L240).

### 16. Access Control
* **Finding:** Unauthorized parties invoking privileged state-mutating endpoints or on-chain instructions.
* **Severity:** High
* **Affected Component:** [`apps/api/src/middleware/auth.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/middleware/auth.rs), [`programs/equity_vault/src/instructions`](file:///home/seam/Desktop/project/equity-catalyst/programs/equity_vault/src/instructions)
* **Evidence:** Constant-time `secure_equals` in API auth middleware; on-chain `constraint = keeper.key() == vault.authority @ VaultError::UnauthorizedKeeper`.
* **Remediation:** All administrative endpoints (`/vaults`, `/policies`, `/events`, `/dbc/configure`) require `x-admin-key` validated with constant-time equality. On-chain instructions verify vault authority signers.
* **Verification:** [`test_secure_equals`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/middleware/auth.rs#L45) and contract access control tests.

### 17. Rate Limiting
* **Finding:** Unauthenticated or authenticated endpoint spam could exhaust API resources and database connection pools.
* **Severity:** Medium
* **Affected Component:** [`apps/api/src/middleware/rate_limit.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/middleware/rate_limit.rs)
* **Evidence:** In-memory sliding window / token bucket rate limiter tracking requests per client IP.
* **Remediation:** Configured via `Config::rate_limit_requests_per_minute` (default 120 RPM, configurable per environment). Exceeding requests receive `429 Too Many Requests`.
* **Verification:** [`test_rate_limiter_allows_under_limit_and_blocks_burst`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/middleware/rate_limit.rs#L95) and [`test_rate_limiter_recovers_after_window`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/middleware/rate_limit.rs#L112).

### 18. Observability
* **Finding:** Absence of standardized health check probes and system introspection prevents container orchestrators (Kubernetes/ECS) from detecting deadlocks or unhealthy downstreams.
* **Severity:** Medium
* **Affected Component:** [`apps/api/src/routes/health.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/routes/health.rs)
* **Evidence:** Implemented `/health` (liveness), `/ready` (readiness with Postgres/Redis ping), and `/health/detailed` (full subsystem health audit).
* **Remediation:** Endpoints return proper HTTP status codes (`200 OK` vs `503 Service Unavailable`) for automated container orchestration.
* **Verification:** Integration tests verify `/health` and `/ready` response schemas and error codes.

### 19. Metrics
* **Finding:** Lack of lightweight telemetry endpoint for monitoring active subscriptions, memory footprint, and operating parameters.
* **Severity:** Medium
* **Affected Component:** [`apps/api/src/routes/metrics.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/routes/metrics.rs)
* **Evidence:** No `/metrics` route was previously registered.
* **Remediation:** Implemented `GET /metrics` reporting service uptime, Linux process RSS memory, active market data subscriptions, configured risk bounds, and execution mode.
* **Verification:** [`test_metrics_endpoint`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/production_hardening_test.rs#L179-L197) verifies status 200 and schema payload.

### 20. Structured Logs
* **Finding:** String-interpolated logging obscures machine readability and indexing in log aggregators (e.g. Datadog, CloudWatch).
* **Severity:** Low
* **Affected Component:** [`apps/api/src/main.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/main.rs), [`apps/api/src/services`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/services)
* **Evidence:** Transitioned to structured field key-values: `info!(cluster = %config.solana_cluster, environment = %config.environment, ...)`.
* **Remediation:** Initialized `tracing_subscriber` with JSON-ready formatting and env-filter support.
* **Verification:** Confirmed structured fields throughout execution logging.

### 21. Error Classification
* **Finding:** Internal errors must not leak internal implementation details, file system paths, or raw stack traces to API consumers.
* **Severity:** High
* **Affected Component:** [`apps/api/src/error.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/error.rs), [`apps/api/src/engines/pipeline_safety.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/pipeline_safety.rs)
* **Evidence:** `ApiError` implements `IntoResponse` with standardized JSON error payloads `{"error": "...", "code": ...}` and appropriate HTTP status codes (400, 401, 403, 404, 409, 429, 500, 503).
* **Remediation:** Strongly typed domain errors: `PolicyRejectionReason`, `PlannerError`, `SignerError`, `MarketDataError`, and `ConfigError`.
* **Verification:** Tested in [`test_pipeline_failure_error_codes_and_status`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/pipeline_safety.rs#L318).

### 22. Graceful Shutdown
* **Finding:** Abrupt termination of the API server drops active database connections, leaves in-flight HTTP requests unfinished, and fails to release Redis connections cleanly.
* **Severity:** Medium
* **Affected Component:** [`apps/api/src/main.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/main.rs)
* **Evidence:** `axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?`.
* **Remediation:** Wired signal handlers for both `SIGINT` (Ctrl+C) and Unix `SIGTERM`, allowing active HTTP requests to complete before closing listeners and connection pools.
* **Verification:** Verified signal handling logic in `main.rs`.

### 23. Configuration Validation
* **Finding:** System starting with invalid slippage parameters, missing database URLs, or negative trade size limits.
* **Severity:** High
* **Affected Component:** [`apps/api/src/config.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/config.rs)
* **Evidence:** `Config::validate()` verifies:
  1. `max_slippage_bps > 0` and within environment limits ($\le 100$ in Dev/Testnet, $\le 100$ in Mainnet).
  2. `max_trade_size_usd > 0`.
  3. `rate_limit_requests_per_minute > 0`.
  4. Environment-specific constraints (valid RPC URLs, required database URL).
* **Remediation:** Server fails closed immediately on startup if `config.validate()` fails.
* **Verification:** [`test_config_validation_rejects_invalid_slippage`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/production_hardening_test.rs#L77-L90).

### 24. Development/Test Secrets Accidentally Reaching Production
* **Finding:** Risk that default development credentials (`"catalyst-admin-secret-dev"`) or mock localnet signers are deployed to Mainnet.
* **Severity:** Critical
* **Affected Component:** [`apps/api/src/config.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/config.rs), [`apps/api/src/engines/decision_engine/signer/dev_test.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/decision_engine/signer/dev_test.rs)
* **Evidence:** `Config::validate()` rejects `"catalyst-admin-secret-dev"`, empty keys, or keys with $< 16$ characters in `Environment::Mainnet`. `DevTestSigner` is explicitly segregated.
* **Remediation:** Strict validation error `ConfigError::DevSecretInProduction` prevents application startup if dev credentials are detected in production profile.
* **Verification:** [`test_config_validation_rejects_dev_secret_in_mainnet`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/production_hardening_test.rs#L36-L61).

### 25. Hardcoded Feed IDs
* **Finding:** Hardcoded Pyth price feed hex IDs break across Pyth cluster upgrades, new asset registrations, or feed deprecations.
* **Severity:** High
* **Affected Component:** [`apps/api/src/services/asset_watcher.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/services/asset_watcher.rs), [`db/migrations/011_asset_market_data.sql`](file:///home/seam/Desktop/project/equity-catalyst/db/migrations/011_asset_market_data.sql)
* **Evidence:** Dynamic feed discovery via `AssetSubscriptionWatcher` and database `asset_market_data` mapping table.
* **Remediation:** Feed IDs are dynamically loaded from database registries and normalized via `normalize_feed_id()`; zero hardcoded feed IDs remain in trading logic.
* **Verification:** Verified dynamic subscription flow in [`apps/api/tests/end_to_end_safety_pipeline_test.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/end_to_end_safety_pipeline_test.rs).

### 26. Hardcoded Private Keys
* **Finding:** Risk of private keys or fallback byte arrays (e.g. `[42u8; 32]`) hardcoded in production source files.
* **Severity:** Critical
* **Affected Component:** [`apps/api/src/engines/decision_engine/signer`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/decision_engine/signer)
* **Evidence:** Codebase grep search confirms zero hardcoded private key byte arrays in production code paths.
* **Remediation:** `KeypairSigner` requires external file path or secret loader; `DevTestSigner` generates random keypairs or is restricted to dev tests.
* **Verification:** Codebase-wide static audit confirmed complete absence of embedded private keys.

### 27. `f64` Usage in Financial / Execution Calculations
* **Finding:** Floating-point `f64` arithmetic in spot cash reserve calculation can suffer from IEEE 754 precision drift, non-deterministic rounding across CPU architectures, and catastrophic cancellation.
* **Severity:** High
* **Affected Component:** [`apps/api/src/engines/risk_engine/mod.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/risk_engine/mod.rs), [`apps/api/src/services/market_data_store.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/services/market_data_store.rs)
* **Evidence:** `risk_engine/mod.rs` previously had `(total_portfolio_usd as f64 * (policy.min_cash_bps as f64 / 10_000.0)).round() as u64`.
* **Remediation:** Replaced with deterministic integer arithmetic using 128-bit scaling and half-up rounding:
  ```rust
  let required_cash_usd = ((total_portfolio_usd as u128 * policy.min_cash_bps as u128 + 5_000) / 10_000) as u64;
  ```
  Prices throughout the execution pipeline are represented as integer micro-USD (6 decimals) and token minor units.
* **Verification:** [`test_deterministic_integer_solvency_calculation`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/production_hardening_test.rs#L169-L177) confirms integer math with half-up rounding precision.

### 28. Symbol-Based Asset Identity
* **Finding:** Ticker symbols (e.g. `"AAPL"` or `"USDC"`) are non-unique and susceptible to collision, spoofing, or cross-chain ambiguity.
* **Severity:** Critical
* **Affected Component:** [`crates/shared/src/models/asset.rs`](file:///home/seam/Desktop/project/equity-catalyst/crates/shared/src/models/asset.rs), [`apps/api/src/services/market_data_store.rs`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/services/market_data_store.rs)
* **Evidence:** Asset identities are indexed primarily by canonical `asset_id` (e.g. `"backed:AAPLx"`) and verified on-chain via SPL token `Pubkey` (mint address).
* **Remediation:** All execution plans, quotes, risk rules, and Anchor instructions operate strictly on Solana token mint `Pubkey` addresses. Symbols are treated strictly as informational display labels.
* **Verification:** Anchor instruction [`execute_action.rs`](file:///home/seam/Desktop/project/equity-catalyst/programs/equity_vault/src/instructions/execute_action.rs#L91-L98) verifies `input_mint.key()` and `output_mint.key()` against `vault.asset_mint` and `compliance.asset_mint`.

### 29. Fail-Open Shariah Screening
* **Finding:** If Shariah compliance screening fails, is missing, or is outdated, trades could execute on non-halal assets if the system fails open.
* **Severity:** Critical
* **Affected Component:** [`programs/equity_vault/src/instructions/execute_action.rs`](file:///home/seam/Desktop/project/equity-catalyst/programs/equity_vault/src/instructions/execute_action.rs), [`apps/api/src/engines/risk_engine`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/src/engines/risk_engine)
* **Evidence:** On-chain instruction explicitly checks:
  ```rust
  require!(compliance.status == COMPLIANCE_STATUS_APPROVED, VaultError::AssetNotApproved);
  require!(clock.unix_timestamp < compliance.valid_until, VaultError::ComplianceExpired);
  ```
* **Remediation:** The entire pipeline fails closed. Missing records, revoked status, or expired compliance timestamps immediately halt execution both on-chain (`VaultError::AssetNotApproved`, `VaultError::ComplianceExpired`) and off-chain (`PolicyRejectionReason::ShariahRejected`).
* **Verification:** Tested in [`test_temporal_compliance_and_status`](file:///home/seam/Desktop/project/equity-catalyst/programs/equity_vault/tests/dex_cpi_execution_test.rs#L38) and [`test_failure_case_asset_deactivated_before_execution`](file:///home/seam/Desktop/project/equity-catalyst/apps/api/tests/end_to_end_safety_pipeline_test.rs#L168).

### 30. Arbitrary CPI Program Invocation
* **Finding:** Malicious keeper accounts passing an arbitrary or untrusted program ID as `dex_program` could hijack vault PDA authority seeds and drain vault funds.
* **Severity:** Critical
* **Affected Component:** [`programs/equity_vault/src/dex/mod.rs`](file:///home/seam/Desktop/project/equity-catalyst/programs/equity_vault/src/dex/mod.rs), [`programs/equity_vault/src/instructions/execute_action.rs`](file:///home/seam/Desktop/project/equity-catalyst/programs/equity_vault/src/instructions/execute_action.rs)
* **Evidence:** Account constraint on `dex_program`:
  ```rust
  #[account(
      constraint = is_authorized_dex_program(&dex_program.key()) @ VaultError::UnauthorizedDexProgram
  )]
  pub dex_program: UncheckedAccount<'info>,
  ```
* **Remediation:** Vault PDA signer seeds are only dispatched to statically authorized DEX programs:
  1. Jupiter V6 (`JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4`)
  2. Meteora Dynamic Bonding Curve (`Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB`)
  3. Controlled Test Mock DEX (in testing profiles)
  Any other program ID is rejected on-chain prior to CPI dispatch.
* **Verification:** [`test_authorized_dex_whitelist_enforcement`](file:///home/seam/Desktop/project/equity-catalyst/programs/equity_vault/tests/dex_cpi_execution_test.rs#L67) verifies unauthorized DEX program IDs are rejected with `UnauthorizedDexProgram`.

---

## Verification Test Summary

The production hardening measures are validated across the project test suites:

1. **`apps/api/tests/production_hardening_test.rs` (8 tests):**
   - Config validation rejecting dev secrets in Mainnet
   - Config validation rejecting localhost RPC in Mainnet
   - Config validation rejecting out-of-bounds slippage
   - Connection URL password sanitization
   - Idempotency tracker memory pruning
   - Deterministic integer cash reserve calculation
   - `/metrics` observability endpoint verification
   - Axum 1MB request body limit enforcement

2. **`apps/api/tests/end_to_end_safety_pipeline_test.rs` (10 tests):**
   - End-to-end multi-stage safety pipeline
   - Rejection on stale oracle
   - Rejection on wrong feed
   - Rejection on expired DEX quote
   - Rejection on excessive slippage
   - Rejection on duplicate execution (idempotency)
   - Rejection on unavailable signer
   - Rejection on asset deactivation / revoked Shariah approval
   - Rejection on negative balance delta / failed on-chain execution

3. **`programs/equity_vault/tests/dex_cpi_execution_test.rs` (8 tests):**
   - On-chain DEX whitelist enforcement
   - Real balance delta output calculation
   - Replay protection via execution PDA seeds
   - Shariah compliance approval and temporal validation
   - Strict token mint pairing checks

**All 26 integration tests and 46 library tests compile cleanly and pass.**
