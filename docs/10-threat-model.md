# 10. Threat Model & Risk Matrix

## 1. Threat Modeling Framework

This threat model evaluates Equity Catalyst using the STRIDE framework adapted for decentralized financial infrastructure on Solana:
* **Assets at Risk**: Depositor SPL tokens (USDC, tokenized equities), vault LP shares, policy configuration parameters, historical audit records.
* **Threat Agents**: Malicious external actors, compromised API servers, MEV bots/searchers, malicious vault managers, adversarial oracle nodes.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          STRIDE THREAT MAPPING                              │
│                                                                             │
│   Spoofing           ──► Forged event payloads / fake oracle prices         │
│   Tampering          ──► Manipulating policy parameters or swap routes       │
│   Repudiation        ──► Denial of triggered trades or audit trail wiping   │
│   Information Leak   ──► Leaking rebalance intent to front-running searchers │
│   Denial of Service  ──► Spamming API queues / exhaust RPC rate limits      │
│   Elevation of Priv  ──► Non-authority attempting emergency pause / drain   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Threat Analysis Matrix

| ID | Threat Vector | Target Asset | Severity | Current Mitigation | Residual Risk | Recommended Enhancement |
|---|---|---|---|---|---|---|
| **T-01** | **Unauthorized Fund Drainage** | Vault SPL Reserves | **CRITICAL** | Anchor PDA seeds govern token transfers. Transfers require PDA signer seeds. | Low | Multi-sig withdrawal approvals for large redemptions. |
| **T-02** | **Malicious Policy Update** | Vault Risk Parameters | **HIGH** | `update_policy` enforces `require_keys_eq!(vault.authority, authority.key())`. | Medium (Manager key compromise) | Timelock contract for policy parameter modifications. |
| **T-03** | **Oracle Manipulation / Flash Attacks** | Rebalancing Orders | **HIGH** | Pyth confidence interval validation (`conf < 0.02 * price`). | Medium (Low liquidity tokens) | TWAP price validation combined with Pyth feeds. |
| **T-04** | **Stale Oracle Exploitation** | Portfolio Valuation | **MEDIUM** | OracleService verifies fetch freshness. | Medium (RPC lag) | On-chain clock assertion in contract bytecode. |
| **T-05** | **Event Queue Flooding (DoS)** | Axum API / Redis | **MEDIUM** | Axum payload limits and JSON schema validation. | Medium (Open endpoint) | API key authentication & IP rate limiting (Tower). |
| **T-06** | **MEV Front-running & Sandwiches**| Swap Execution | **HIGH** | Price impact capped at 10%; default slippage 0.50%. | High on mainnet DEX swaps | Private mempool submission (Jito Block Engine). |
| **T-07** | **Replay of Processed Events** | Portfolio Rebalance | **HIGH** | Idempotency guard in PolicyWorker checks `status == 'PROCESSED'`. | Low | DB unique composite constraint over event parameters. |
| **T-08** | **First Depositor Inflation Attack** | Share Accounting | **HIGH** | Contract asserts `require!(shares_to_mint > 0)`. | Low | Permanently burn dead shares on vault initialization. |
| **T-09** | **Excessive Single-Trade Exposure** | Vault NAV | **MEDIUM** | RiskEngine enforces `MAX_SINGLE_TRADE_BPS = 1000` (10% max). | Low | Dynamic slippage curve based on pool liquidity. |
| **T-10** | **Keeper Private Key Compromise** | Execution Signer | **HIGH** | Signer has zero withdrawal authority; only triggers pre-flight checked actions. | Low | Cloud HSM / Turnkey isolated signing agent. |

---

## 3. Attack Scenarios & Deep-Dive Analysis

### Scenario A: Adversarial Manager Tries to Change Policy to 100% LTV
* **Attacker Profile**: Rogue vault authority or compromised manager laptop.
* **Attack Method**: The attacker submits an `update_policy` transaction setting `max_ltv_bps = 20000` ($200.00\%$) to take catastrophic leverage and default.
* **Contract Defense**:
  ```rust
  require!(max_ltv_bps <= MAX_BPS, EquityVaultError::InvalidBasisPoints);
  ```
  The Anchor program halts execution with `InvalidBasisPoints` because $20,000 > 10,000$. The transaction reverts without mutating state.

### Scenario B: Attacker Injects Phony Event to Trigger NVDA Sell-off
* **Attacker Profile**: External user calling `POST /events`.
* **Attack Method**: Attacker crafts an event stating `event_type: "earnings_miss"`, `sentiment_score: -0.99` for NVDA.
* **System Defense**:
  1. `PolicyEngine` matches `PolicyRule::EarningsMiss` and proposes a $-500\text{ bps}$ reduction in NVDA.
  2. `RiskEngine` evaluates proposed sell trade:
     * Does the trade exceed 10% single trade limit? (Checked).
     * Does it breach stop conditions? (Checked).
  3. `PolicyWorker` evaluates in **Dry-Run Mode**: logs decision to console and saves audit log with status `LOGGED`.
  4. **No live transaction is broadcast** to Solana. The malicious input produces only a benign dry-run audit record.
