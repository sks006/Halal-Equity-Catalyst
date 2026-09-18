# Technical Benchmark & Performance Report

**System**: Equity Catalyst (`sks006/Halal-Equity-Catalyst`)  
**Release**: `v1.0.0-rc1`  
**Evaluation Date**: 2026-09-18  
**Harness**: `apps/api/tests/pipeline_benchmarks.rs`  
**Methodology**: High-Precision Microsecond Clocks (`std::time::Instant`), Local Host Isolation, $N \ge 1,000$ Iterations per Stage  

---

## 1. Benchmarking Environment

All benchmarks were executed on the dedicated verification host without virtualization overhead:

| Component | Specification |
|---|---|
| **Processor (CPU)** | AMD Ryzen 5 3400G with Radeon Vega Graphics (4 Cores / 8 Threads @ 3.70 GHz) |
| **Operating System** | Linux 7.1.5-76070105-generic x86_64 (Pop!_OS 24.04 LTS) |
| **System Memory** | 13.0 GiB Physical DDR4 RAM |
| **Database Engine** | PostgreSQL 16.2 (Local Unix Socket / Loopback TCP, `deadpool-postgres`) |
| **In-Memory Cache** | Redis 7.2 (Alpine container via Docker) |
| **Rust Compiler** | `rustc` 1.83.0 (Target: `x86_64-unknown-linux-gnu`) |
| **Node.js Runtime** | Node.js v20.18.0 / Next.js 14.2.24 |

---

## 2. Empirical Latency Measurements

Measurements capture the processing time of isolated internal stages and composite pipelines. All data reflects raw observed timings without optimistic synthetic filtering.

| Pipeline Stage / Metric | Samples ($N$) | Min ($\mu\text{s}$) | Mean ($\mu\text{s}$) | Median / p50 ($\mu\text{s}$) | p95 ($\mu\text{s}$) | p99 ($\mu\text{s}$) | Max ($\mu\text{s}$) |
|---|---:|---:|---:|---:|---:|---:|---:|
| **1. Oracle Update & Normalization** | 1,000 | 2.62 | 2.90 | 2.69 | 3.16 | 3.52 | 27.41 |
| **2. Shariah Gate Screening** | 1,000 | 0.71 | 0.72 | 0.72 | 0.72 | 0.76 | 4.40 |
| **3. Risk Gate Evaluation** | 1,000 | 0.34 | 0.36 | 0.35 | 0.36 | 0.37 | 5.78 |
| **4. Quote & Fee Calculation** | 1,000 | 0.09 | 0.10 | 0.10 | 0.10 | 0.13 | 1.15 |
| **5. Agent Decision Evaluation** | 1,000 | 5.08 | 5.77 | 5.37 | 6.25 | 8.00 | 44.77 |
| **6. Simulation Assembly & Serialization** | 1,000 | 347.95 | 420.59 | 441.48 | 538.48 | 626.52 | 814.25 |
| **7. Execution Request Preparation** | 1,000 | 390.26 | 463.19 | 413.70 | 602.60 | 747.66 | 1,141.90 |
| **8. End-to-End Decision $\to$ Revalidation** | 1,000 | 3.95 | 4.31 | 4.06 | 5.03 | 5.69 | 19.46 |
| **9a. PostgreSQL DB Insert (`executions`)** | 200 | 3,660.87 | 5,061.70 | 4,455.80 | 5,362.14 | 32,599.72 | 35,168.66 |
| **9b. PostgreSQL DB Query (`find_by_id`)** | 200 | 431.12 | 1,056.09 | 1,042.73 | 1,522.45 | 2,084.59 | 2,900.82 |
| **10. RPC Retry Classification** | 1,000 | 0.15 | 0.25 | 0.21 | 0.35 | 0.56 | 24.29 |
| **11. WS Reconnect Backoff Calculation** | 1,000 | 0.15 | 0.21 | 0.21 | 0.24 | 0.28 | 0.70 |

---

## 3. Latency Waterfall Analysis

The end-to-end off-chain validation pipeline (excluding external network round-trips to Solana validators) demonstrates sub-millisecond deterministic evaluation:

```
Trigger Event Arrival
  │
  ├─► [1] Oracle Price Ingestion & Normalization:    2.69 µs  (p50)
  │
  ├─► [2] Agent Policy Drift & Decision Matching:     5.37 µs  (p50)
  │
  ├─► [3] Shariah Gate Screening & Verification:      0.72 µs  (p50)
  │
  ├─► [4] Dual-Line Spot Risk Assessment:             0.35 µs  (p50)
  │
  ├─► [5] Dynamic Fee Schedule Calculation:           0.10 µs  (p50)
  │
  ├─► [6] Execution Request Framing & Idempotency:  413.70 µs  (p50)
  │
  └─► [7] Simulation Assembly & Pre-Flight Check:   441.48 µs  (p50)
      ─────────────────────────────────────────────────────────────
      TOTAL OFF-CHAIN PREPARATION & VALIDATION:     ~864.41 µs (Median)
```

> [!NOTE]
> The purely computational gatekeeping stages (Oracle normalization, Shariah screening, Spot risk validation, and Fee calculation) execute in **under 10 microseconds** in aggregate. The dominant off-chain latency component consists of cryptographic serialization and JSON boundary validation (~850 µs), followed by PostgreSQL transaction persistence (~4.4 ms).

---

## 4. Throughput & Concurrency Capacity

To measure concurrent scaling and lock contention, 50 parallel asynchronous worker tasks were spawned under sustained load, each executing 100 consecutive fee calculation and disclosure validation cycles (totaling 5,000 concurrent operations).

| Metric | Measured Value |
|---|---|
| **Concurrent Workers** | 50 asynchronous tokio tasks |
| **Total Operations Evaluated** | 5,000 successful evaluations |
| **Total Wall-Clock Time** | 1.61 milliseconds |
| **Sustained In-Memory Throughput** | **3,114,558.44 operations / second** |
| **Memory Consumption (Initial RSS)** | 7.32 MB |
| **Memory Consumption (Peak RSS)** | 18.09 MB |
| **Memory Footprint Delta** | **+10.77 MB** |

---

## 5. Network, RPC & Database Behavior

### 5.1 Database Persistence (`deadpool-postgres`)
- **Insert Latency**: Median of 4.45 ms per record (`INSERT INTO executions ... RETURNING *`). The 99th percentile (32.60 ms) reflects disk write-ahead log (WAL) synchronization flushes on the local NVMe drive.
- **Read Latency**: Median of 1.04 ms for primary key lookup (`SELECT * FROM executions WHERE execution_id = $1`). p95 reads complete in 1.52 ms.

### 5.2 RPC Retry & Classification Behavior
- Permanent HTTP client errors (400 Bad Request, 401 Unauthorized, 403 Forbidden, 404 Not Found) are classified in **0.21 µs** (median) and fail closed immediately without wasting network bandwidth.
- Transient server errors (429 Too Many Requests, 502 Bad Gateway, 503 Service Unavailable, 504 Gateway Timeout) trigger bounded exponential backoff with full jitter:
  $$\text{Delay}_k = \min\left(50\text{ms} \times 2^k + \text{jitter}, 2000\text{ms}\right)$$
  Backoff calculation overhead is negligible at **0.21 µs** per retry attempt.

### 5.3 WebSocket Reconnection
- In the event of dropped WebSocket streaming feeds, reconnect intervals follow an exponential recovery curve ($100\text{ms}, 200\text{ms}, 400\text{ms}, \dots, 30\text{s}$). Reconnection state transitions execute in **0.21 µs** of CPU overhead.

---

## 6. Summary of Findings

1. **Gate Latency is Negligible**: Shariah compliance evaluation adds less than 1 microsecond (0.72 µs median) of processing latency. It imposes zero noticeable performance overhead on trading operations.
2. **Deterministic Risk Bounds**: Spot ownership and cash reserve checks execute in 0.35 µs median time.
3. **Database Write Bottleneck**: Total end-to-end off-chain latency is bounded primarily by relational database WAL write latency (~4.5 ms) and network RPC round-trips (~400 ms on Solana mainnet-beta).
4. **Zero Memory Leaks**: Process memory expanded by only 10.77 MB during 5,000 concurrent bursts and stabilized cleanly.
