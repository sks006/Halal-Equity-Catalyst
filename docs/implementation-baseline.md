# Phase 0 Implementation Baseline & Audit Report

**Date:** 2026-09-23  
**Phase:** PHASE 0 — Baseline & Safety  
**Status:** COMPLETE (Baseline Established & Documented)

---

## 1. Repository Structure

The workspace is a monorepo combining a Rust Cargo workspace, Solana Anchor smart contracts, a Next.js 14 frontend, a TypeScript SDK, database migrations, and operational scripts.

### Workspace Members (`Cargo.toml`)
- **`crates/shared`**: Canonical pure domain types, asset identity, risk models, mathematical validation, redaction utilities, and Shariah screening governance logic.
- **`apps/api`**: Axum-based REST and WebSocket API server hosting execution engines, policy workers, risk management, and domain services.
- **`programs/equity_vault`**: Anchor on-chain program (`8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH`) managing vault PDA accounts, deposit/withdraw shares, policy limits, and execution logging receipts.
- **`integrations/pyth`**: Pyth Network Hermes v2 client, raw and normalized price domain models, staleness detection, and price feed registry.
- **`integrations/jupiter`**: Jupiter v6 Swap routing API client, quote evaluation, and mock client modes for unit testing.
- **`integrations/solana`**: Solana RPC client with fallback endpoints, retry logic, WebSocket log listeners, and Anchor client wrapper.

### External Workspace Packages & Directories
- **`apps/web`**: Next.js 14 (App Router) web application providing the executive dashboard, vault management, live market monitoring, error state previews, and agent replay UI.
- **`sdk`**: TypeScript SDK (`@equity-catalyst/sdk`) providing programmatic access to vaults, policies, events, Meteora DBC curves, and Pyth Lazer streaming.
- **`demo-data`**: Static JSON fixtures (`nvda-earnings-event.json`, `sample-vault.json`, `sample-policy.json`, `sample-portfolio.json`, `sample-position.json`, `demo-script.json`).
- **`db/migrations`**: PostgreSQL database schema migrations `001_initial.sql` through `009_dead_letters.sql`.
- **`scripts`**: Operational automation scripts (`initialize.ts`, `launch-real-pool.ts`, `replay-demo.ts`, `simulate-equity-curve.ts`, `verify-mainnet.ts`).
- **`tests`**: Anchor client tests (`equity_vault.ts`) and end-to-end integration tests (`full_pipeline_e2e.test.ts`, `equity_discovery_curve.test.ts`).

---

## 2. Existing Pyth Architecture

### Components & Responsibilities
- **Pyth Client (`integrations/pyth/client.rs`)**:
  - `PythClient`: Wraps `reqwest::Client` with 10-second default timeout. Connects to Pyth Hermes HTTP endpoint (`/v2/updates/price/latest?parsed=true`).
  - Implements an in-memory `mock_mode` (`Arc<RwLock<bool>>`) and `mock_prices` store for synthetic offline testing.
  - Queries are batch-optimized via `get_latest_prices(&[&str])` and individual feed lookups `get_latest_price(&str)`.
- **Price Types (`integrations/pyth/types.rs`)**:
  - `PythRawPrice`: String price, confidence, exponent (`i32`), and publish timestamp (`i64`). Custom deserializer supports string or numeric payload fields.
  - `ParsedPriceFeed`: Structure containing `id`, `price: PythRawPrice`, and optional `ema_price: PythRawPrice`.
  - `NormalizedPrice`: Domain price representation containing:
    - `price_usd: f64` (calculated as `raw_price * 10^expo`)
    - `price_scaled: u64` (fixed-point micro-USD, 6 decimals: `$1.00 = 1_000_000`)
    - `conf_usd: f64` (confidence band in USD)
    - `is_stale: bool` (calculated against `max_staleness_secs`)
- **Feed Registry (`integrations/pyth/feeds.rs`)**:
  - `PythFeedRegistry`: In-memory bidirectional map (`RwLock<HashMap>`) mapping tickers (`SOL`, `BTC`, `ETH`, `USDC`, `AAPL`, `AAPLX`, `NVDA`, `NVDAX`, `TSLA`, `MSFT`, `SPY`, `SPYX`) to canonical 64-character Hermes feed hex strings (without `0x` prefix).
- **Oracle Domain Service (`apps/api/src/services/oracle_service.rs`)**:
  - `OracleService`: Enforces confidence interval ratio checks (`conf_usd / price_usd <= max_confidence_ratio`, default 5.00%) and max staleness checks (`max_staleness_secs`, default 120s in service constructor, though tightened in specific engine gates).
  - Normalizes asset prices and synchronizes valuations directly to PostgreSQL via `PortfolioRepository`.
- **API Routes & AppState**:
  - Endpoint: `GET /oracle/price/:symbol` mapped via `apps/api/src/routes/oracle.rs`.
  - `AppState` (`apps/api/src/state.rs`) holds an optional `Arc<OracleService>`, initialized during application startup in `apps/api/src/lib.rs` and `apps/api/src/main.rs`.
- **Configuration (`apps/api/src/config.rs`)**:
  - `pyth_hermes_url`: Defaults to `"https://hermes.pyth.network"` across development, testnet, and mainnet profiles.

---

## 3. Existing Execution & Signing Architecture

### Signing Implementation (`apps/api/src/engines/decision_engine/signer.rs`)
- **Cryptographic Primitives**: Uses standard Ed25519 asymmetric signing via Solana SDK (`solana_sdk::signature::Keypair`).
- **Canonical Framing**: Trade parameters are serialized into `CanonicalExecutionPayload` with a 28-byte domain separation prefix (`b"EQUITY_CATALYST_EXECUTION_V1"`), hashed via SHA-256 (`payload.digest()`), and signed by the backend keeper keypair.
- **Key Loading & Fallback**:
  - `ExecutionSigner::load_or_generate(path_str)` attempts to read an Ed25519 keypair from a JSON byte array file on disk.
  - **Fallback Key**: If the file does not exist, it issues a warning and initializes a deterministic fallback keypair using `let default_seed = [42u8; 32];`.
- **Read-Only Guards**:
  - The API configuration defaults `read_only: true` for both Testnet and Mainnet profiles.
  - `AnchorClient` (`integrations/solana/anchor_client.rs`) initializes with `submit_transactions: false` and explicitly rejects transaction submissions with `SolanaError::TransactionsDisabled` unless unlocked.

### On-Chain Anchor Execution (`programs/equity_vault/src/instructions/execute_action.rs`)
- **Action Constraints**: Restricts `action_type` to `1` (Spot Swap) or `4` (Emergency Exit). Rejects leverage, lending, or margin.
- **Compliance Pre-check**: Checks `compliance.status == COMPLIANCE_STATUS_APPROVED` and temporal expiration `clock.unix_timestamp < compliance.valid_until`.
- **Actual DEX Execution**:
  - **No DEX Swap Occurs**: The program creates an `Execution` PDA account (`EXECUTION_SEED`) recording `execution_id`, `input_mint`, `output_mint`, `input_amount`, and sets `actual_output_amount = min_output_amount`.
  - **No Cross-Program Invocation (CPI)**: There is zero invocation to Jupiter, Meteora, Raydium, or SPL Token transfer instructions inside `execute_action`. Token balances are not transferred.

---

## 4. Existing Shariah Screening Architecture

### Implementation Details (`crates/shared/src/shariah/`)
- **Status Machine (`types.rs`)**:
  - Canonical states: `Pending`, `Approved`, `Rejected`, `Expired`, `Revoked`. Only `Approved` permits trading (`can_trade()`).
- **Business Category Classification (`screening.rs`)**:
  - Intrinsically prohibited sectors (`Alcohol`, `Gambling`, `ConventionalFinance`, `Tobacco`, `Weapons`, `AdultEntertainment`) yield `BusinessClassification::Prohibited`.
  - Technology, Healthcare, Manufacturing, and `Other(String)` yield `BusinessClassification::RequiresReview`.
  - **Fail-Closed Handling**: `screen_business_activity()` rejects unreviewed categories with `ShariahRejectionReason::BusinessClassificationRequiresReview`. Unspecified categories without descriptions fail closed.
- **Financial Screening Ratios (`screening.rs` & `policy.rs`)**:
  - Metrics record `ShariahFinancialMetrics`:
    - `debt_ratio_bps: u32` (Interest-bearing debt / denominator)
    - `interest_bearing_cash_ratio_bps: u32` (Cash & interest deposits / denominator)
    - `receivables_cash_ratio_bps: Option<u32>` (Receivables + cash / denominator)
    - `impure_income_ratio_bps: u32` (Non-operating impure revenue / total revenue)
    - `denominator_method: DenominatorMethod` (`CurrentMarketCap`, `AverageMarketCapMonths(u8)`, `TotalAssets`)
  - Policy limits use `BasisPoints` (`u16` wrapper) with explicit `ThresholdComparison` semantics (`StrictLessThan` vs `LessThanOrEqual`).
- **Purification Model (`screening.rs`)**:
  - Implements `PurificationAssessment` with `purification_ratio_bps: u32`, `dividend_amount_minor: u64`, `purification_amount_minor: u64`, and `net_permissible_amount_minor: u64`.
  - Calculated via checked integer arithmetic in `calculate_purification()`. Decoupled from asset qualification (assets under 5% impure income remain Approved, but incidental income must be donated).
- **Decision Engine Gate Integration (`apps/api/src/engines/decision_engine/mod.rs`)**:
  - Default constructor `DecisionEngine::new` initializes `canonical_shariah_registry()`, which delegates to `test_fixture_shariah_registry()` with hardcoded synthetic financial ratios and test fixture evidence hashes (`test_fixture_hash:cert_backed_nvda`, etc.).

---

## 5. Existing Tests & Verification Baseline

### Test Suite Execution
- **`crates/shared`**: 80 tests passing (77 unit tests + 3 property-based math tests in `math_property_test.rs`).
- **`integrations/pyth`**: 6 unit tests passing.
- **`integrations/jupiter`**: 4 unit tests passing.
- **`integrations/solana`**: 14 unit tests passing (including HTTP 503 and HTTP 429 mock failover tests).
- **`programs/equity_vault`**: 1 unit test passing.
- **`apps/api` (lib)**: 38 unit tests passing.

---

## 6. Existing Compilation & Test Failures

### 1. Formatting (`cargo fmt --all -- --check`)
- **Status:** FAILED
- **Files Affected:**
  - `crates/shared/src/shariah/mod.rs`
  - `crates/shared/src/shariah/screening.rs`
  - `crates/shared/src/shariah/types.rs`
  - `crates/shared/tests/math_property_test.rs`
- **Cause:** Multi-line function call argument layout differences against `rustfmt`.

### 2. Compilation (`cargo check --workspace`)
- **Status:** PASSED (0 errors, 0 warnings).

### 3. Test Suite Workspace Run (`cargo test --workspace`)
- **Status:** FAILED (Exit Code 101)
- **Failing Tests:** 17 tests in `apps/api/tests/adversarial_attack_test.rs` (and integration test files requiring a database):
  - `test_attack_01_unknown_asset_rejected_no_signing`
  - `test_attack_02_fake_provider_rejected_no_signing`
  - `test_attack_03_expired_compliance_rejected_no_signing`
  - `test_attack_04_revoked_compliance_rejected_no_signing`
  - `test_attack_05_wrong_mint_rejected_no_signing`
  - `test_attack_06_fake_evidence_hash_rejected_no_signing`
  - `test_attack_07_unowned_sell_rejected_no_signing`
  - `test_attack_08_unfunded_buy_rejected_no_signing`
  - `test_attack_09_stale_pyth_price_rejected_no_signing`
  - `test_attack_10_large_price_deviation_rejected_no_signing`
  - `test_attack_11_bad_slippage_rejected_no_signing`
  - `test_attack_12_duplicate_execution_idempotency_rejected`
  - `test_attack_13_simultaneous_duplicate_execution_race_rejected`
  - `test_attack_14_pause_during_execution_aborted_no_signing`
  - `test_attack_15_rpc_failure_fails_closed_no_signing`
  - `test_attack_16_database_failure_fails_closed_no_signing`
  - `test_attack_18_unauthorized_compliance_and_policy_update_rejected`
- **Cause:** Integration tests attempt to connect to PostgreSQL at `localhost:5432/equity_catalyst`. PostgreSQL was inactive in the local environment during workspace test execution (`"Database connection failed: error connecting to server"`).

---

## 7. Security Issues Discovered

1. **Deterministic Fallback Private Key Seed**:
   - `ExecutionSigner::load_or_generate` falls back to `[42u8; 32]` if the specified file does not exist. If production runs without configuring the key file, transactions are signed by a publicly known seed.
2. **Hardcoded Test Fixture Shariah Registry in Decision Engine**:
   - `DecisionEngine::new` invokes `canonical_shariah_registry()`, which returns `test_fixture_shariah_registry()` with synthetic ratios and test hashes (`test_fixture_hash:...`), rather than requiring a verified registry.
3. **Execution Anchor Instruction Does Not Perform Swaps**:
   - `execute_action` records the execution receipt but contains zero CPI to DEX swap programs (e.g. Meteora or Jupiter) and does not transfer token funds.
4. **Integration Test Environment Dependency**:
   - Test suites in `apps/api/tests/` panic when PostgreSQL is offline instead of gracefully mocking or skipping database-dependent tests.

---

## 8. Items Explicitly Deferred to Later Phases

- **Phase 1 (Canonical Asset Registry)**: Formalizing asset data models, registries, and production asset definitions.
- **Phase 2–7 (Pyth & Market Data Stream)**: Pyth SSE real-time client, dynamic subscription manager, and market data store.
- **Phase 8–11 (API, DEX Quotes, Risk & Planner)**: Live DEX quote integrations, multi-venue routing, and execution planning.
- **Phase 12 (Real Cryptographic Signer)**: Removing the deterministic fallback key `[42u8; 32]` and enforcing strict keyfile/HSM requirements.
- **Phase 13 (Anchor DEX CPI Execution)**: Implementing genuine on-chain Cross-Program Invocations (CPI) to DEX pools.
- **Phase 14–18 (Purification, Failure Safety, Production Hardening)**: Full execution measurement, purification disbursement, and production deployment gates.
