# 25. Current State vs. Target State Gap Analysis

## 1. Prioritization Framework

This gap analysis categorizes architectural and implementation deltas using standard institutional severity tiers:
* **`P0`**: Critical blocker for security, data integrity, or core production deployment.
* **`P1`**: Important architectural enhancement required for high-throughput institutional operation.
* **`P2`**: Performance optimization, developer experience improvement, or non-critical refactoring.
* **`P3`**: Long-term protocol roadmap evolution (e.g. governance, advanced lending markets).

---

## 2. Gap Analysis Matrix

| Area | Current Implementation | Problem / Architectural Defect | Target State | Priority |
|---|---|---|---|---|
| **On-Chain DEX Swaps** | Live execution suppressed (`status = 'LOGGED'`). No on-chain `execute_action` instruction. | Vault cannot autonomously execute rebalancing swaps directly on Solana. | Implement Anchor `execute_action` instruction invoking Jupiter CPI with min-output checks. | **P0** |
| **Share Inflation Floor** | Asserts `shares_to_mint > 0`, but has no dead shares burn. | Vulnerable to ERC-4626 style inflation griefing by first depositor donating dust tokens. | Burn 1,000 dead shares to `Pubkey::default()` during `initialize_vault`. | **P0** |
| **Oracle Timestamp Assertion** | Timestamp verified in off-chain Rust service, but not checked on-chain. | If an off-chain keeper is compromised or feeds stale prices, on-chain contract cannot detect it. | Enforce `Clock::get()?.unix_timestamp - price.publish_time <= 60` in contract CPI. | **P0** |
| **API Authentication & Rate Limiting** | Public endpoints without auth tokens or rate limiting. | Vulnerable to event queue flooding, DoS, and spam attacks on `POST /events`. | Implement JWT/API key authentication for keepers and IP-based rate limiting via Tower. | **P1** |
| **Worker Database Queries** | `PolicyWorker` executes 3 separate SQL roundtrips per event. | Unnecessary query latency ($> 15\text{ ms}$) on high event volume. | Single joined SQL query fetching vault, policy, and positions in one round-trip. | **P1** |
| **Redis Connection Management** | Creates new multiplexed connection handle per polling loop iteration. | Socket churn, CPU allocation overhead, potential connection pool exhaustion. | Maintain a single persistent `MultiplexedConnection` handle inside `PolicyWorker`. | **P1** |
| **Signer Key Security** | Reads local JSON keypair file from filesystem or deterministic seed. | Server compromise (RCE) exposes execution signing key. | Integrate AWS CloudHSM, HashiCorp Vault, or Turnkey MPC for signing execution manifests. | **P1** |
| **On-Chain Borrow & Repay** | Structs defined in Anchor state, but instructions are not coded in contract. | Vault collateral cannot yet be borrowed against for dynamic hedging or leverage. | Implement `borrow` and `repay` Anchor instruction handlers with dynamic LTV checks. | **P2** |
| **Pyth Price In-Memory Caching** | HTTP GET dispatched to Hermes on every client oracle price request. | Upstream Hermes rate limits (HTTP 429) during high frontend traffic. | In-memory 500ms TTL cache using `moka` or `dashmap` for hot asset prices. | **P2** |
| **Metrics & Prometheus Telemetry** | Tracing logs emitted to stdout, but no Prometheus endpoint. | Inability to alert on risk rejections, queue backlog, or RPC latency in Grafana. | Implement `/metrics` endpoint exporting Prometheus counters and histograms. | **P2** |
| **Tokenized Equity Provider SDKs** | `integrations/xstocks` and `integrations/ondo` are empty directories. | Lack of direct primary issuance mint/burn integration with RWA issuers. | Build native connectors to Backed Finance and Ondo tokenized treasury APIs. | **P3** |
| **DAO Timelock Governance** | Vault authority can update risk policy parameters immediately. | Manager can make sudden adverse parameter changes without depositor notice. | Implement a 24-hour timelock smart contract for non-emergency policy parameter changes. | **P3** |
