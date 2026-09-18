# Capabilities & Verification Matrix

This matrix establishes the truthful operational state of all major capabilities within Equity Catalyst. Every claim is strictly grounded in actual code, local tests, integration suites, or on-chain queries.

### Status Legend
- **`MAINNET_VERIFIED`**: Account, program, or feed data queried directly and verified from Solana `mainnet-beta` RPC or Pyth Hermes.
- **`DEVNET_VERIFIED`**: Anchor program deployment and PDAs verified on Solana `devnet`.
- **`LOCALLY_VERIFIED`**: Validated by automated Rust unit, integration, or property test suites.
- **`IMPLEMENTED`**: Code is complete and checked by compiler/linter; awaiting live deployment.
- **`BLOCKED`**: Depends on live capital wallet deployment or external mainnet actions (zero fabricated data allowed).

---

| Capability | Implementation Location | Local Tests | Integration Tests | Devnet | Mainnet | Public Evidence / Identifier | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Tokenized Equity Mint (NVDAx)** | `crates/shared/src/asset.rs` | `asset::tests` (4 tests) | `scripts/verify-mainnet.ts` | Configured | **PASS** | Mint: `Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh` (Token-2022, 679 bytes) | `MAINNET_VERIFIED` |
| **Pyth Hermes Oracle Feed** | `integrations/pyth/src/client.rs` | `pyth_test.rs` (6 tests) | `oracle_service_test.rs` (3 tests) | Configured | **PASS** | Feed ID: `b1073854ed24cbc755dc527418f52b7d271f6cc967bbf8d8129112b18860a593` (`Equity.US.NVDA/USD`) | `MAINNET_VERIFIED` |
| **Meteora DBC Program** | `integrations/solana/src/dbc.rs` | `crates/shared/src/liquidity.rs` | `scripts/verify-mainnet.ts` | Configured | **PASS** | Program ID: `dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN` (Executable: true) | `MAINNET_VERIFIED` |
| **Anchor Smart Vault** | `programs/equity_vault/src/lib.rs` | `tests/anchor/equity_vault.ts` | `solana_integration_test.rs` (14 tests) | **PASS** | Pending Deployment | Program ID: `8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH` | `DEVNET_VERIFIED` |
| **Deterministic Shariah Screening Engine** | `crates/shared/src/shariah/screening.rs` | `shariah::tests` (26 tests) | `math_property_test.rs` (3 tests) | N/A (Off-chain) | N/A | AAOIFI Standard No. 21 quantitative ratio tests (< 30% debt, < 30% cash, < 5% impure) | `LOCALLY_VERIFIED` |
| **AssetRegistry Security Gate** | `crates/shared/src/shariah/registry.rs` | `shariah::tests::test_shariah_asset_registry_full_security_gate` | `domain_services_test.rs` | N/A | N/A | Blocks unapproved, expired, or unverified assets from entry | `LOCALLY_VERIFIED` |
| **Spot Ownership Validation** | `crates/shared/src/risk.rs` | `risk::tests::test_validate_spot_ownership_rules` | `engine_matrix_test.rs` | N/A | N/A | Rejects oversells and zero-balance sales before execution | `LOCALLY_VERIFIED` |
| **Spot Funding Validation** | `crates/shared/src/risk.rs` | `risk::tests::test_validate_spot_funding_rules` | `engine_matrix_test.rs` | N/A | N/A | Enforces settled cash balance >= gross trade requirement (0% leverage) | `LOCALLY_VERIFIED` |
| **Pre-Signing Emergency Pause Guard** | `apps/api/src/services/execution_engine_service.rs` | `security_hardening_test.rs` (8 tests) | `execution_failure_audit_test.rs` | N/A | N/A | Aborts execution immediately before Step 5 signing if vault paused | `LOCALLY_VERIFIED` |
| **Signer Key Isolation** | `apps/api/src/engines/decision_engine/signer.rs` | `redaction::tests` (2 tests) | `security_hardening_test.rs` | N/A | N/A | Zero keys exposed to AI, API routes, or serialized responses | `LOCALLY_VERIFIED` |
| **Deterministic Fee Breakdown** | `crates/shared/src/fees.rs` | `fees::tests` (6 tests) | `math_property_test.rs` | N/A | N/A | Invariant `total_fee == pool_fee + platform_fee`; rejects drift | `LOCALLY_VERIFIED` |
| **Constant-Time Admin Authentication** | `apps/api/src/middleware/auth.rs` | `security_hardening_test.rs` | `api_endpoints_test.rs` | N/A | N/A | Constant-time API key verification (`x-admin-key`); rejects missing/invalid with 401 | `LOCALLY_VERIFIED` |
| **Sliding-Window Rate Limiting** | `apps/api/src/middleware/rate_limit.rs` | `security_hardening_test.rs` | `api_endpoints_test.rs` | N/A | N/A | Bounded timestamp tracking; rejects burst traffic with HTTP 429 | `LOCALLY_VERIFIED` |
| **Dead-Letter Queue Logging** | `apps/api/src/repositories/dead_letter_repository.rs` | `security_hardening_test.rs` | PostgreSQL integration | N/A | N/A | Persists unprocessable / malformed events in `dead_letters` table | `LOCALLY_VERIFIED` |
| **Worker Supervisor Panic Recovery** | `apps/api/src/workers/supervisor.rs` | `supervisor::tests` | Event worker pipeline | N/A | N/A | Bounded exponential backoff with sliding restart window | `LOCALLY_VERIFIED` |
| **Next.js Web Dashboard** | `apps/web/src/` | Type-check passed | `pnpm --prefix apps/web build` | Connects to RPC | Connects to RPC | 18 static & dynamic routes compiled | `LOCALLY_VERIFIED` |
| **Live Meteora Mainnet Pool Account** | On-chain DBC Account | N/A | `scripts/verify-mainnet.ts` | Not Deployed | Blocked | Awaiting operational wallet SOL funding for mainnet creation | `BLOCKED` |
| **Live Mainnet Trade Execution** | Solana Transaction Signature | N/A | `scripts/verify-mainnet.ts` | Not Submitted | Blocked | Zero fabricated transaction signatures | `BLOCKED` |
