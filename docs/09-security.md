# 09. Security Architecture & Defense-in-Depth

## 1. Trust Boundaries & Authority Model

Equity Catalyst isolates cryptographic permissions across four distinct authority boundaries to guarantee defense-in-depth:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            TRUST BOUNDARY MAP                               │
│                                                                             │
│  [User Authority (External Wallet)]                                         │
│    └── Owns User ATA and UserShares PDA. Can ONLY deposit & withdraw.       │
│                                                                             │
│  [Vault Authority (Manager Keypair)]                                        │
│    └── Created the Vault PDA. Can update Policy parameters or toggle pause. │
│        CANNOT unilaterally seize user assets.                               │
│                                                                             │
│  [Isolated Execution Signer (Backend Keeper)]                               │
│    └── Signs ExecutionRequests. Held strictly in memory / isolated daemon.  │
│        Can ONLY trigger pre-approved swap instructions.                     │
│                                                                             │
│  [Program Authority (Solana Anchor PDA)]                                    │
│    └── Controls vault asset tokens via PDA bump seed derivation.            │
│        Enforces share calculation invariants and risk ceilings.             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. In-Depth Security Audit by Vector

### 2.1 Replay Attacks on Event Ingestion & Execution

* **Risk**: Replaying a historic earnings beat event to force duplicate rebalancing trades.
* **Attack Scenario**: An attacker captures a historic HTTP request to `POST /events` detailing a $+8\%$ NVDA surprise and replays it 50 times against the API. If unmitigated, the system would repeatedly buy NVDA, exhausting USDC reserves.
* **Impact**: Severe capital misallocation, excessive transaction fees, and portfolio concentration breach.
* **Current Protection**:
  1. `EventModel` primary key is a deterministic or generated `UUIDv4`.
  2. Idempotency check in `apps/api/src/workers/policy_worker.rs:84`:
     ```rust
     if event.status == "PROCESSED" {
         warn!(event_id = %event.event_id, "Idempotency guard: Event has already been processed");
         return Err(ApiError::BadRequest(...));
     }
     ```
* **Missing Protection**: Ingestion endpoint currently allows inserting new event rows with identical payloads unless client-provided UUID deduplication is enforced.
* **Recommended Mitigation**: Enforce a composite uniqueness constraint on `(vault_address, event_type, source, (payload->>'surprise_pct'))` over a rolling 24-hour window in PostgreSQL.

---

### 2.2 Oracle Manipulation & Stale Price Exploitation

* **Risk**: Flash loan manipulation of low-liquidity oracles, or trading against stale price feeds.
* **Attack Scenario**: An attacker manipulates an on-chain AMM price for a tokenized stock, or an RPC node serves a 2-hour-old Pyth price. The policy engine evaluates a false stop-loss trigger and panic-sells assets at a steep discount.
* **Impact**: Direct capital drain through arbitrageurs buying distressed vault assets.
* **Current Protection**:
  1. `integrations/pyth` verifies price confidence intervals (`price.conf`) and rejects prices with confidence $> 2\%$ of spot price.
  2. Decimal normalization verifies exponent boundaries:
     ```rust
     if price.expo < -12 || price.expo > 12 {
         return Err(PythError::InvalidDecimalPrecision(...));
     }
     ```
* **Missing Protection**: Maximum age timestamp validation on price feeds is currently handled in application code rather than asserted via on-chain contract clock constraints.
* **Recommended Mitigation**: Enforce a strict `require!(Clock::get()?.unix_timestamp - price.publish_time <= 60)` check before any swap instruction can execute on-chain.

---

### 2.3 Slippage & Front-Running / MEV Sandwiches

* **Risk**: Excessive slippage on Jupiter DEX swaps or predatory MEV sandwich attacks on Solana.
* **Attack Scenario**: A keeper broadcasts a rebalance transaction buying $\$50,000$ of NVDAx with $5\%$ slippage tolerance. A Jito searcher sandwiches the transaction, extracting value from vault depositors.
* **Impact**: Permanent economic loss to vault NAV.
* **Current Protection**:
  1. `QuoteExecutionService` inspects Jupiter `priceImpactPct` and rejects quotes exceeding `policy.rebalance_threshold_bps` (capped at 1000 bps = 10%).
  2. Pure shared crate sets hard constant `MAX_PERMISSIBLE_SLIPPAGE_BPS = 300` (3.00%).
  3. Default slippage in quote requests is clamped to $50\text{ bps}$ ($0.50\%$).
* **Missing Protection**: On-chain Anchor program does not currently enforce minimum output amount validation directly in contract bytecode.
* **Recommended Mitigation**: When on-chain swap instructions are implemented, the `execute_action` instruction MUST take `min_output_amount: u64` and verify `actual_output >= min_output_amount` post-CPI.

---

### 2.4 Integer Overflow & Share Inflation Attacks

* **Risk**: First depositor inflation attack (ERC-4626 style) where an attacker deposits 1 unit, donates a large amount to the vault, and dilutes subsequent depositors via rounding down to 0 shares.
* **Attack Scenario**: An attacker initializes a vault, deposits 1 lamport/atomic unit (receiving 1 share), and transfers $\$10,000$ directly to the vault token account. The next user deposits $\$5,000$. The formula $\lfloor \frac{5,000 \times 1}{10,000} \rfloor = 0$ mints 0 shares to the user.
* **Impact**: Total theft of second depositor's capital.
* **Current Protection**:
  1. `programs/equity_vault/src/instructions/deposit.rs:36`:
     ```rust
     require!(shares_to_mint > 0, EquityVaultError::ZeroSharesMinted);
     ```
     The transaction reverts immediately if calculated shares equal zero!
  2. Calculations use checked 128-bit unsigned arithmetic (`u128`), preventing integer overflow.
* **Missing Protection**: Virtual offset shares (dead shares) are not currently burned to `Pubkey::default()` on vault initialization.
* **Recommended Mitigation**: Permanently burn $1,000$ initial shares to a dead address during `initialize_vault` to establish an unbreakable inflation floor.

---

### 2.5 Execution Signer Key Isolation & Backend Compromise

* **Risk**: Compromise of the server hosting `apps/api` leading to unauthorized fund drainage.
* **Attack Scenario**: An attacker achieves remote code execution (RCE) on the Axum server and attempts to steal private keys to drain vaults.
* **Impact**: Catastrophic loss if keys have withdrawal authority.
* **Current Protection**:
  1. **Separation of Concerns**: The `ExecutionSigner` (`apps/api/src/engines/decision_engine/signer.rs`) does NOT hold withdrawal keys. It only signs execution authorizations.
  2. User deposits can ONLY be withdrawn by the specific `user` keypair that owns the corresponding `UserShares` PDA.
  3. The Anchor program prevents any keeper key from directing assets to an arbitrary external address; assets must flow between vault-controlled token accounts.
* **Missing Protection**: Execution keys are currently read from local filesystem json files or deterministic seeds.
* **Recommended Mitigation**: Integrate an on-chain multi-signature threshold (Squads Protocol) or AWS CloudHSM / HashiCorp Vault for signing execution manifests.
