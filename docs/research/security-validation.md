# Security Validation & Adversarial Attack Matrix

**System**: Equity Catalyst (`sks006/Halal-Equity-Catalyst`)  
**Release**: `v1.0.0-rc1`  
**Evaluation Date**: 2026-09-18  
**Test Suite**: `apps/api/tests/adversarial_attack_test.rs`  
**Security Invariant**:
$$\text{ATTACK} \longrightarrow \text{DETERMINISTIC REJECTION} \longrightarrow \text{ZERO SIGNING} \longrightarrow \text{NO BROADCAST}$$

---

## 1. Executive Security Architecture

Equity Catalyst enforces an institutional security perimeter where no AI agent, external user, or automated trigger can authorize or sign a Solana transaction without satisfying multi-stage deterministic barriers.

```
[External Trigger / AI Agent Proposal]
                  │
                  ▼
       [1. Authentication & Rate Limit]  ──(Invalid/Burst)──► REJECT 401/429
                  │
                  ▼
       [2. Shariah Asset Gate]           ──(Impermissible)──► REJECT (No Signing)
                  │
                  ▼
       [3. Spot Ownership Gate]          ──(Short / Oversell)─► REJECT (No Signing)
                  │
                  ▼
       [4. Spot Funding Gate]            ──(Debt / Margin)──► REJECT (No Signing)
                  │
                  ▼
       [5. Dual-Line Risk Engine]        ──(Limit Breach)──► REJECT (No Signing)
                  │
                  ▼
       [6. Oracle Freshness Gate]        ──(Stale / Drift)──► REJECT (No Signing)
                  │
                  ▼
       [7. Fee Disclosure Gate]          ──(Fee Drift)─────► REJECT (No Signing)
                  │
                  ▼
       [8. Pre-Signing Emergency Pause]  ──(Paused)────────► REJECT (No Signing)
                  │
                  ▼
       ═════════════════════════════════════════════════════
       [CRYPTOGRAPHIC SIGNING BOUNDARY — ExecutionSigner]
       ═════════════════════════════════════════════════════
                  │
                  ▼
       [Solana Network Broadcast]
```

---

## 2. Adversarial Attack Matrix (18 Vectors Tested)

All 18 adversarial attack vectors were tested in automated integration tests under `apps/api/tests/adversarial_attack_test.rs`. In every case, the attack was rejected deterministically, and cryptographic signing was never invoked (`signing_count == 0`).

| Vector ID | Attack Description | Injected Attack Payload / Condition | Rejecting Component | Rejection Error Code | Signing Invocations | Broadcast Count | Test Result |
|---|---|---|---|---|---:|---:|:---:|
| **ATK-01** | **Unknown Asset** | Unregistered ticker `UNKNOWN_MEME_COIN` | Shariah Asset Registry | `AssetNotRegistered` | **0** | **0** | **PASS** |
| **ATK-02** | **Fake Provider** | Untrusted provider reference `SCAM_TOKEN_FAKE_PROVIDER` | Provider Resolver | `None` / `ResolutionKind::None` | **0** | **0** | **PASS** |
| **ATK-03** | **Expired Compliance** | Asset review `expires_at = now - 10` | Shariah Execution Gate | `ComplianceExpired` | **0** | **0** | **PASS** |
| **ATK-04** | **Revoked Compliance** | Asset status revoked after proposal approval | Shariah Execution Gate | `AssetNotApproved (Revoked)` | **0** | **0** | **PASS** |
| **ATK-05** | **Wrong Mint** | Attacker mint substituted in swap destination | Anchor Instruction / Simulation | `ComplianceMintMismatch` | **0** | **0** | **PASS** |
| **ATK-06** | **Fake Evidence Hash** | Compliance record with empty/whitespace hash | Shariah Screening Engine | `InvalidEvidenceHash` | **0** | **0** | **PASS** |
| **ATK-07** | **Unowned SELL** | SELL 500 units with 0 units in vault (*Bay' ma la Yamlik*) | Ownership Gate | `ProhibitedLeverageOrShort` | **0** | **0** | **PASS** |
| **ATK-08** | **Unfunded BUY** | BUY requiring $50k with $10k cash or 2.0x leverage | Spot Funding Gate | `ProhibitedLeverageOrShort` | **0** | **0** | **PASS** |
| **ATK-09** | **Stale Pyth Price** | Oracle publish timestamp 10 minutes in the past | Oracle Service | `StalePrice` | **0** | **0** | **PASS** |
| **ATK-10** | **Large Price Deviation** | Oracle price jumps 50% from proposal quote | Slippage & Quote Gate | `PriceDeviationExceeded` | **0** | **0** | **PASS** |
| **ATK-11** | **Bad Slippage** | Client requests 50% allowable slippage (5,000 bps) | Policy Engine Limits | `SlippageExceeded` | **0** | **0** | **PASS** |
| **ATK-12** | **Duplicate Execution** | Replay of identical `execution_id` sequentially | Idempotency Gate | Deduplicated / Cached Outcome | **0** | **0** | **PASS** |
| **ATK-13** | **Simultaneous Race** | Parallel threads submitting identical `execution_id` | Database Unique Constraint | Unique Violation Handled | **0** | **0** | **PASS** |
| **ATK-14** | **Pause During Execution** | Vault emergency pause flipped immediately pre-signing | Execution Engine Step 5 | `VaultEmergencyPaused` | **0** | **0** | **PASS** |
| **ATK-15** | **RPC Outage** | Solana RPC unreachable (TCP connection refused) | Solana Service | `RpcConnectionFailed (Fail Closed)` | **0** | **0** | **PASS** |
| **ATK-16** | **Database Failure** | PostgreSQL socket closed during lock acquisition | Execution Repository | `DatabaseConnectionFailed` | **0** | **0** | **PASS** |
| **ATK-17** | **Worker Crash** | Unhandled worker panic unwind during loop execution | Worker Supervisor | Caught via `AssertUnwindSafe` | **0** | **0** | **PASS** |
| **ATK-18** | **Unauthorized Admin** | State mutation attempted without `x-admin-key` | Auth Middleware | `HTTP 401 Unauthorized` | **0** | **0** | **PASS** |

---

## 3. Deep Dive into Critical Attack Vectors

### 3.1 Unowned SELL (Bay' ma la Yamlik) — Vector ATK-07
- **Attack Strategy**: The attacker attempts to profit from an expected market downturn by submitting a SELL order for 500 shares of `NVDAx` when the vault possesses 0 shares, or possesses only 200 shares.
- **Defense Mechanism**: `validate_spot_ownership` inspects the exact settled vault position balance. If `available_quantity == 0` or `requested_quantity > available_quantity`, it returns `ValidationError::ProhibitedLeverageOrShort`.
- **Cryptographic Result**: Execution terminates at Stage 6 of the off-chain pipeline. The transaction message is never constructed, the keypair is never invoked, and no instruction reaches the Solana mempool.

### 3.2 Unfunded BUY & Leverage Prohibition — Vector ATK-08
- **Attack Strategy**: The attacker submits a BUY request exceeding the vault's settled cash reserves, or requests 2.0x margin financing.
- **Defense Mechanism**: `validate_spot_funding` verifies:
  1. `leverage_multiple <= 1.0` and not NaN.
  2. `borrowed_amount_usd == 0` (zero interest-bearing debt).
  3. `available_settled_cash_usd >= required_cash_usd`.
- **Cryptographic Result**: Rejection is immediate. Zero margin trading or lending pools are integrated.

### 3.3 Temporal Compliance Expiry — Vector ATK-03
- **Attack Strategy**: An asset was previously approved, but its statutory quarterly compliance review expired 10 seconds before trade execution.
- **Defense Mechanism**: Off-chain `revalidate_shariah_and_spot` evaluates `now < eligibility.expires_at` using fresh system timestamps. On-chain, the Anchor program evaluates `Clock::get()?.unix_timestamp < compliance.valid_until`.
- **Cryptographic Result**: Both boundaries fail closed independently. Neither stale off-chain caches nor lapsed on-chain PDAs allow trade construction.

### 3.4 Pre-Signing Emergency Pause — Vector ATK-14
- **Attack Strategy**: An administrative vault pause is issued while an execution request is already in-flight (after simulation, immediately prior to Step 5 signing).
- **Defense Mechanism**: Step 5 re-queries `vault_repo.find_by_address` directly before delegating to `ExecutionSigner`. If `vault.is_paused == true`, execution aborts with `ApiError::BadRequest("Vault is paused")`.
- **Cryptographic Result**: The signing key is never accessed.

---

## 4. Cryptographic Proof of Signer Isolation

In the test suite, an isolated `ExecutionSigner` with an `Arc<AtomicUsize>` counter was monitored across all 18 attack vectors:

```rust
// Verified across all test cases:
assert_eq!(
    harness.signing_spy_count.load(Ordering::SeqCst), 
    0, 
    "Signer must never be invoked when validation fails before signing boundary"
);
```

**Total Attack Scenarios**: 18  
**Total Rejections**: 18 (100% Deterministic)  
**Total Signing Invocations**: 0  
**Total False Approvals**: 0  
