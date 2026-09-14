# 17. Debugging Architecture & Diagnostic Guide

## 1. The Senior Engineering Debugging Discipline

When debugging complex asynchronous, event-driven, or on-chain systems, shotgun debugging (randomly changing multiple files at once) introduces regressions and obscures root causes. Equity Catalyst mandates a disciplined scientific methodology:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       THE DISCIPLINED DEBUGGING LOOP                        │
│                                                                             │
│          ┌──────────┐                                                       │
│          │ ONE BUG  │ ──► Identify the exact symptom and failing state      │
│          └────┬─────┘                                                       │
│               │                                                             │
│               ▼                                                             │
│       ┌───────────────┐                                                     │
│       │ONE HYPOTHESIS │ ──► Formulate one falsifiable root cause            │
│       └───────┬───────┘                                                     │
│               │                                                             │
│               ▼                                                             │
│       ┌───────────────┐                                                     │
│       │ONE SMALL CHG  │ ──► Modify the single minimal line or block of code │
│       └───────┬───────┘                                                     │
│               │                                                             │
│               ▼                                                             │
│          ┌──────────┐                                                       │
│          │ ONE TEST │ ──► Run the exact unit or integration test to prove   │
│          └──────────┘                                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Pipeline Diagnostic Checkpoints

To locate where a transaction or event stalls, inspect telemetry at each discrete checkpoint in the lifecycle:

```mermaid
graph TD
    CP1[1. EVENT RECEIVED?<br/>Check: SELECT * FROM events WHERE event_id = '...']
    CP2[2. POLICY MATCHED?<br/>Check: tracing log PolicyRule matched in rules.rs]
    CP3[3. SIGNAL GENERATED?<br/>Check: PolicySignal emitted with target delta]
    CP4[4. PORTFOLIO STATE VALID?<br/>Check: sum(weights) == 10,000 bps & NAV > 0]
    CP5[5. TRADES CALCULATED?<br/>Check: calculate_rebalance_plan trade deltas]
    CP6[6. RISK CHECKS PASSED?<br/>Check: RiskAssessment outcome & rejection reason]
    CP7[7. DECISION CREATED?<br/>Check: ExecutionRequest approved == true]
    CP8[8. AUDIT PERSISTED?<br/>Check: SELECT * FROM executions WHERE decision_id = '...']

    CP1 --> CP2 --> CP3 --> CP4 --> CP5 --> CP6 --> CP7 --> CP8
```

### Boundary Telemetry Table

| Checkpoint | Log Field or Query | Expected Signal | Common Failure Root Cause |
|---|---|---|---|
| **1. Event Ingested** | `POST /events` | HTTP 201, `status = 'PENDING'` | Invalid JSON syntax or missing `vault_address`. |
| **2. Worker Popped** | `debug!("Popped event from Redis queue")` | Event picked up by `PolicyWorker` | Redis connection error or worker thread crashed. |
| **3. Policy Matched** | `match_rule` | `PolicyRule::EarningsBeat` | `sentiment_score` below threshold ($0.50$). |
| **4. Weights Verified** | `calculate_target_allocation` | Weights sum to $10,000\text{ bps}$ | Duplicate symbols or missing USDC cash reserve. |
| **5. Risk Evaluated** | `RiskEngine::evaluate_proposed_trades` | `RiskAssessment::Approved` | Trade $> 10\%$ of NAV or exposure $> \text{max\_position}$. |
| **6. Pre-Flight Run** | `validate_decision_preflight` | Pre-flight OK | Vault is paused or policy is marked inactive. |
| **7. Decision Logged** | `executions.status` | `'LOGGED'` | Postgres connection pool exhaustion. |

---

## 3. Common Troubleshooting Playbooks

### Playbook A: "My event was ingested, but no rebalance occurred."
1. **Checkpoint**: Inspect the event row in PostgreSQL:
   ```sql
   SELECT event_id, status, detected_at, processed_at FROM events ORDER BY detected_at DESC LIMIT 5;
   ```
2. **If `status == 'PENDING'`**:
   * The `PolicyWorker` is either not running or blocked. Inspect worker logs:
     ```bash
     RUST_LOG=debug cargo run -p equity-catalyst-api
     ```
   * Verify Redis connectivity: `redis-cli ping` (expect `PONG`).
3. **If `status == 'PROCESSED'`**:
   * Inspect the corresponding execution record:
     ```sql
     SELECT execution_id, action, status, error_message FROM executions WHERE event_id = '<EVENT_ID>';
     ```
   * If `status == 'LOGGED'` and `action == 'NOOP'`: The drift was smaller than `policy.rebalance_threshold_bps`.
   * If `action == 'REJECTED'`: Inspect `error_message` for the exact risk boundary violation.

---

### Playbook B: "Quote evaluation returned `approved: false`."
1. **Checkpoint**: Inspect the response from `POST /quotes/evaluate`.
2. **Examine `rejection_reason`**:
   * `"Price impact 150 bps breaches maximum allowed limit 100 bps"`: The pool lacks depth for the requested size. Reduce `amount_in` or widen `rebalance_threshold_bps`.
   * `"Exposure violation: ExceedsMaxPosition"`: The purchase would push the asset's concentration over `policy.max_position_bps`.
   * `"Vault operations are currently paused"`: Unpause the vault via `emergency_exit(is_paused = false)`.

---

### Playbook C: "Anchor integration test fails with `ZeroSharesMinted`."
1. **Formulate Hypothesis**: The deposit amount is too small relative to total deposits and shares, resulting in integer truncation to zero.
2. **Inspect Code**: In `deposit.rs:36`:
   ```rust
   let shares_to_mint = (amount as u128 * total_shares as u128) / total_deposits as u128;
   require!(shares_to_mint > 0, EquityVaultError::ZeroSharesMinted);
   ```
3. **Verify with Pure Math Test**: Run `cargo test -p equity-catalyst-shared test_share_calculation`.
4. **Fix**: Increase the deposit amount in the test fixture so that $\text{shares\_to\_mint} \ge 1$.
