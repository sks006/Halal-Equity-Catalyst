# Security Architecture & Risk Controls Summary

Review Date: 2026-09-18
Scope: Whole Workspace (`equity-catalyst-shared`, `equity-catalyst-api`, `equity-catalyst-solana`, `equity-catalyst-pyth`, `equity-catalyst-jupiter`, `equity-vault`)

---

## 1. Core Architectural Boundary

Equity Catalyst enforces a strict unidirectional pipeline that ensures untrusted external inputs and advisory AI models cannot access private keys, execute transactions, or bypass risk controls:

```
[Market Signal / Earnings Event]
              ↓
    [AI Advisory Proposal]  <-- Quarantined: Zero keys, Zero signing authority
              ↓
  [Deterministic Shariah Gate] <-- AAOIFI Standard 21 ratio validation & status check
              ↓
     [Spot Risk Engine]   <-- 10% Cash reserve, 0% Debt, Spot ownership verification
              ↓
    [Pre-Flight Simulation]  <-- Evaluates slippage, price impact, and liquidity bounds
              ↓
[Pre-Signing Pause Recheck]  <-- Aborts immediately if vault paused or policy deactivated
              ↓
    [Isolated Signer]   <-- Cryptographic key isolation (never leaves server boundary)
              ↓
  [Solana Settlement]   <-- Atomic swap finality on Solana (~400ms)
              ↓
 [PostgreSQL & Redis DB]  <-- Immutable execution record & dead-letter audit queue
```

---

## 2. Tested Security Invariants

The following 10 immutable security controls have been implemented and verified via automated regression test suites (`apps/api/tests/security_hardening_test.rs` and `crates/shared/tests/math_property_test.rs`):

1. **AI Signer Isolation**: The AI agent proposes portfolio adjustments (`AgentProposal`); it has zero access to cryptographic keypairs and cannot broadcast transactions.
2. **Deterministic Shariah Gate**: Unapproved, expired, or non-compliant assets cannot reach execution.
3. **Spot Ownership Enforcement**: Sells are verified against actual vault position balances; naked short selling and overselling fail closed.
4. **Spot Funding Enforcement**: Purchases require settled cash balances; margin, loans, and leverage are permanently prohibited ($0\%$ debt).
5. **No Derivatives**: Futures, perpetual funding rates, and options are structurally rejected by the domain model.
6. **Execution Idempotency**: UUID idempotency keys prevent double execution across sequential retries and concurrent submissions.
7. **Pre-Signing Emergency Pause Guard**: Even if an order passes simulation, if a vault is paused or policy deactivated prior to transaction construction, execution aborts and the signer is never invoked.
8. **Constant-Time Administrative Authentication**: State-mutating API routes require constant-time API key verification (`subtle::ConstantTimeEq`), rejecting invalid credentials with HTTP 401.
9. **Sliding-Window Rate Limiting**: Global sliding-window rate limiters prevent API exhaustion and burst abuse, rejecting excess requests with HTTP 429.
10. **Zero-Secret Logging**: All sensitive data is wrapped in `RedactedSecret<T>` and custom `Debug` formatters, displaying `[REDACTED]` in logs, error payloads, and traces.

---

## 3. Production Deployment Prerequisites

Before deploying to live production with institutional capital:
1. **Multi-Signature Governance**: Transition Anchor vault authority and compliance update authority from single keeper keys to an on-chain multi-sig (e.g. Squads v4).
2. **KMS / HSM Signer Integration**: Connect `ExecutionSigner` to an enterprise hardware security module (HSM) or AWS KMS / GCP Cloud KMS rather than local disk keypairs.
3. **Dedicated RPC Infrastructure**: Secure private, authenticated Solana RPC nodes with dedicated failover providers.
4. **Formal Shariah Board Review**: Engage an accredited Shariah Supervisory Board for formal auditing and *Fatwa* issuance against the production parameters.
