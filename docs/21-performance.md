# 21. Performance & Computational Efficiency Analysis

## 1. Hot Paths & Computational Profile

In an institutional portfolio controller, latency impacts execution price, slippage, and protection against front-running.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          LATENCY PROFILE BREAKDOWN                          │
│                                                                             │
│  [1. Pure Domain Calculations (crates/shared)]                              │
│    └── Basis points, drift, rebalancing math: < 50 microseconds (Pure CPU) │
│                                                                             │
│  [2. Database Persistence (PostgreSQL & Redis)]                             │
│    └── Redis FIFO push/pop: < 1 ms                                          │
│    └── Postgres row insert/query: 1 - 5 ms                                  │
│                                                                             │
│  [3. External REST APIs (Pyth & Jupiter)]                                   │
│    └── Pyth Hermes HTTP quote: 50 - 150 ms                                  │
│    └── Jupiter v6 quote retrieval: 100 - 300 ms                             │
│                                                                             │
│  [4. Solana Consensus & Confirmation]                                       │
│    └── Block production: 400 ms slot time                                   │
│    └── "Confirmed" commitment: 400 - 1,200 ms                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Bottleneck Analysis & Optimization Review

### Optimization 1: Redundant Database Queries in `PolicyWorker`
* **Current Bottleneck**: In `apps/api/src/workers/policy_worker.rs:89-105`, every event triggers 3 sequential database round-trips:
  1. `vault_repo.find_by_address`
  2. `policy_repo.find_by_vault`
  3. `portfolio_repo.list_by_vault`
* **Evidence**: In trace logs, database queries account for $> 80\%$ of total worker processing time before the decision engine is invoked.
* **Expected Impact**: $3\times$ latency reduction in worker pipeline by combining into a single SQL query joining `vaults`, `policies`, and `portfolios`.
* **Complexity Cost**: Low. Requires a single joined query method in `VaultRepository`.

---

### Optimization 2: Redis Connection Pooling
* **Current Bottleneck**: In `PolicyWorker::poll_and_process_next`, the worker requests a connection from the Redis client on every single polling iteration:
  ```rust
  if let Ok(mut conn) = client.get_multiplexed_async_connection().await { ... }
  ```
* **Evidence**: Creating multiplexed async connection handles repeatedly introduces TCP handshake overhead and socket allocations.
* **Expected Impact**: Eliminates connection establishment churn; saves $2 - 5\text{ ms}$ per polling cycle.
* **Complexity Cost**: Minimal. Maintain one long-lived `MultiplexedConnection` handle inside the struct.

---

### Optimization 3: Excessive String Cloning Across Models
* **Current Bottleneck**: `PortfolioModel`, `EventModel`, and `ExecutionModel` clone string fields (`vault_address`, `asset_symbol`, `action`) repeatedly across the service layers.
* **Evidence**: High allocation count in heap profiling during stress tests.
* **Expected Impact**: Memory footprint reduction and cache locality improvement.
* **Complexity Cost**: Low to Medium. Use `Arc<str>` or borrowed string slices (`&str`) within domain engine evaluation pipelines.

---

### Optimization 4: In-Memory Caching for Pyth Price Feeds
* **Current Bottleneck**: `GET /oracle/price/:symbol` calls the external Hermes endpoint via HTTP on every user request.
* **Evidence**: 100 requests/second from web dashboards will trigger Hermes rate limiting (HTTP 429) or latency spikes ($> 200\text{ ms}$).
* **Expected Impact**: Sub-millisecond response times ($< 1\text{ ms}$) and zero upstream rate-limiting risk.
* **Complexity Cost**: Low. Implement a 500ms in-memory cache with `moka` or `dashmap`.
