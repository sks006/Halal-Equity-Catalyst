# System Limitations & Operational Boundaries

**System**: Equity Catalyst (`sks006/Halal-Equity-Catalyst`)  
**Release**: `v1.0.0-rc1`  
**Evaluation Date**: 2026-09-18  
**Target Audience**: Systems Engineers, Institutional Auditors, and Risk Officers  

---

## 1. Overview & Honest Engineering Assessment

Equity Catalyst is engineered to provide institutional, Shariah-gated price discovery for tokenized equities on Solana. However, no production system operates without constraints. This document details the technical boundaries, protocol dependencies, throughput bottlenecks, and operational trade-offs of the architecture.

---

## 2. On-Chain Constraints (Solana & Anchor)

### 2.1 Block Time & Finality Bounds
- **Slot Time**: Solana target slot duration is ~400 ms, with real-world mainnet-beta variance between 380 ms and 600 ms.
- **Commitment Levels**:
  - `processed`: Lowest latency (~400 ms), but subject to fork rollbacks if the slot does not achieve consensus.
  - `confirmed`: 66%+ cluster stake confirmation (~800 ms to 1.2 s). Default for Equity Catalyst state updates.
  - `finalized`: 31+ confirmed blocks (~12 to 15 s). Maximum safety, but unsuitable for sub-second algorithmic trading.
- **Implication**: Any claims of "zero-latency" or "instantaneous" on-chain settlement are physically impossible on public distributed ledgers.

### 2.2 Account Serialization & Write Lock Contention
- Solana executes non-overlapping transactions in parallel across Sealevel runtime threads. However, transactions that write to the same account are strictly serialized.
- In Equity Catalyst:
  - All trades modifying a given `Vault` PDA (`[b"vault", authority, name]`) or `Meteora DBC Pool` account must execute sequentially.
  - If 50 trade proposals target the same vault simultaneously, Solana will process them in serial slots or sequential transactions, creating a transaction queue latency floor of $50 \times \text{slot\_duration} \approx 20\text{ seconds}$ under unbatched load.

### 2.3 Compute Unit (CU) Limits
- Solana transactions have a maximum compute budget of 1,400,000 CU (default 200,000 CU).
- The `execute_action` instruction with compliance verification, SPL token transfers, and Meteora dynamic curve CPI consumes approximately 140,000 to 180,000 CU. Batching more than 4 equity swaps into a single transaction will exceed the compute budget.

---

## 3. Market Data & Oracle Boundaries (Pyth Network)

### 3.1 Hermes HTTP Polling vs. Push Oracles
- Pyth Network publishes off-chain Pythnet price updates to Hermes endpoints.
- Under REST polling (`/v2/updates/price/latest`), update latency is subject to HTTP client connection pools and Hermes cache refresh cycles (~200 ms to 500 ms).
- WebSocket streaming (`/v2/updates/price/stream`) significantly reduces ingestion latency to ~50 ms, but requires persistent socket maintenance, ping/pong heartbeats, and failover reconnect handlers.

### 3.2 Publisher Confidence Interval Degradation
- Pyth prices include a confidence interval ($\pm \sigma$). During real-world equity market halts, earnings announcements, or high-volatility opens (9:30 AM EST), publisher confidence intervals can widen drastically.
- When $\sigma / \text{Price} > \text{MaxConfidenceBps}$, the system rejects automated execution to prevent trading against wide or uncertain spreads.

### 3.3 After-Hours Equity Illiquidity
- Underlying real-world stocks (e.g. NVIDIA, Apple) trade primarily during US equity market hours (9:30 AM – 4:00 PM EST).
- Outside market hours, tokenized equity secondary liquidity on Meteora DBC may diverge from the last published Pyth closing price. The system enforces strict maximum slippage bounds (e.g., 50 bps) to prevent execution against dislocated pools.

---

## 4. Liquidity & AMM Mechanics (Meteora Dynamic Bonding Curves)

### 4.1 Migration Threshold Liquidity Cliff
- Meteora DBC curves operate with a migration threshold (e.g., $100,000 in accumulated USDC liquidity).
- Prior to migration, trades trade directly against the bonding curve formula ($y = f(x)$).
- Once the migration threshold is reached, trading on the DBC halts while liquidity is locked and transferred to a permanent DAMM (Dynamic AMM) pool. During this migration window (~1 to 2 minutes), automated trade execution on that asset will be temporarily blocked.

### 4.2 Maximum Trade Size vs. Price Impact
- Equity tokens initially deployed on DBC curves have modest initial pool reserves (e.g., $20,000 to $50,000).
- Large orders ($> \$5,000$) on low-reserve pools will incur non-linear price impact exceeding 150 bps, triggering automatic rejection by the Risk Engine. Institutional rebalancing must be executed via time-weighted algorithmic slices.

---

## 5. Off-Chain Infrastructure & Architectural Limits

### 5.1 Relational Database Write Contention
- As established in `benchmark-report.md`, in-memory computational checks execute in microseconds (0.72 µs for Shariah gate, 0.35 µs for Risk gate), but PostgreSQL `INSERT` and `UPDATE` queries on `executions` table require 4.45 ms (median) and up to 35 ms (p99 disk sync).
- PostgreSQL write-ahead log (WAL) synchronization is the primary off-chain latency constraint. In high-frequency configurations, unbuffered synchronous persistence must be offloaded to asynchronous write queues or Redis streams.

### 5.2 Single Keeper Keypair Concurrency
- The automated execution worker uses an isolated keeper keypair (`ExecutionSigner`).
- A single Solana keypair can maintain only one active transaction nonce/blockhash sequence per slot. Submitting multiple concurrent transactions from the same signer key without durable nonces can cause blockhash collisions and dropped transactions.
- **Solution**: Multi-worker deployments require a deterministic pool of keeper keypairs.

### 5.3 Clock Drift & NTP Synchronization
- Off-chain temporal compliance checks compare `Utc::now().timestamp()` against `expires_at`.
- On-chain Anchor checks compare `Clock::get()?.unix_timestamp` against `valid_until`.
- While Solana cluster time tracks real-world UTC within a few seconds, significant NTP server drift on the off-chain host could cause false positive rejections at compliance expiry boundaries. Verification servers must run `chrony` or `systemd-timesyncd`.

---

## 6. Failure Recovery & Operational Runbook

| Failure Mode | Impact | Automatic Mitigation | Manual Operator Action |
|---|---|---|---|
| **Primary RPC 503 / Outage** | Inability to fetch slot state or simulate | Automatic failover to secondary endpoints (`solana_fallback_rpc_urls`) with jittered backoff | Verify status of primary RPC provider node |
| **Pyth Hermes Feed Halt** | Stale price rejection for target ticker | Fail closed (`ApiError::BadRequest("Price is stale")`); zero trades executed | Check status of Pyth Network Hermes cluster |
| **Worker Thread Panic** | Potential thread termination | `WorkerSupervisor` catches panic, applies exponential backoff, records restart, and resumes loop | Inspect dead-letter queue table for corrupt payload |
| **Unprocessable Event Ingestion** | Event parsing failure | Event captured in `dead_letters` PostgreSQL table with full payload and error diagnosis | Review and acknowledge via administrative dashboard |
| **Emergency Vault Pause** | Vault halted | All execution requests fail closed immediately at Step 5 signing boundary | Investigate market conditions before issuing unpause instruction |
