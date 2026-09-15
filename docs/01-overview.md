# 01. Executive Overview & Product Vision

## 1. Executive Summary

**Equity Catalyst** is an institutional-grade programmable portfolio controller and autonomous equity vault protocol built on Solana. It enables asset managers, algorithmic trading desks, and retail investors to wrap tokenized equities (e.g. NVDAx, MSFTx, AAPLx) and stablecoins into non-custodial, policy-driven vaults that dynamically rebalance, hedge, and manage risk in response to real-world financial events.

The core thesis of Equity Catalyst is:

> **Tokenized stocks already exist on Solana. Equity Catalyst does not seek to recreate market infrastructure, build another central limit order book, or issue synthetic assets. Instead, it provides the missing programmable governance and automated control plane that transforms passive on-chain equity tokens into autonomous, risk-managed robo-portfolios.**

---

## 2. The Problem Statement

### 2.1 The Passive Liquidity Trap in Real World Assets (RWAs)
Tokenized stocks (issued by entities like Backed, Ondo, or Swarm) exist natively as SPL tokens on Solana. However, today they function primarily as static, passive buy-and-hold assets in personal wallets. Investors who want to manage a diversified equity portfolio face severe friction:
1. **Manual Rebalancing**: Adjusting portfolio allocations in response to quarterly earnings reports, macro announcements, or price momentum requires constant off-chain monitoring and manual swap execution.
2. **Execution Latency & Fragmentation**: By the time an investor reacts to an earnings release, prices have adjusted. There is no automated bridge connecting off-chain financial data (earnings surprise, analyst upgrades, SEC filings) to on-chain execution.
3. **Absence of Enforceable Risk Guardrails**: Off-chain algorithmic bots often hold direct custody or full private-key authority over user funds. If a trading bot experiences an error, anomalous loop, or compromised API key, it can drain user capital or take catastrophic leverage.
4. **Poor Composability with DeFi**: Tokenized equity positions cannot easily be utilized as dynamic collateral for automated hedging or programmatic credit without custom smart contract infrastructure.

---

## 3. Product Boundaries

Equity Catalyst establishes strict boundaries to keep the protocol focused, secure, and performant:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        EQUITY CATALYST BOUNDARY                        │
│                                                                        │
│   IN SCOPE (Core Value Proposition)                                    │
│   ├── Non-custodial SPL vault management with pro-rata share issuance │
│   ├── On-chain policy enforcement (Max LTV, Max Position Exposure)    │
│   ├── Event-driven off-chain evaluation (Earnings, Volatility, Drift)  │
│   ├── Multi-layer risk validation & pre-flight sanity defense          │
│   ├── Isolated execution signing and Jupiter DEX quote routing        │
│   └── Real-time on-chain state observation and portfolio telemetry     │
│                                                                        │
│   OUT OF SCOPE (Leverages Existing Solana Infrastructure)               │
│   ├── DEX / CLOB Matching Engine  ──► Delegated to Jupiter & Phoenix   │
│   ├── Equity Tokenization / Custody ──► Delegated to SPL RWA Issuers   │
│   ├── Primary Market Maker Quotes ──► Delegated to OpenBook / Raydium  │
│   └── Lending Money Markets       ──► Delegated to Kamino / MarginFi   │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 4. The Core Lifecycle

The entire architecture follows a deterministic, closed-loop control cycle:

```
  ┌──────────────┐
  │   OBSERVE    │ ◄─── Solana WebSocket, Pyth Hermes Feeds, SEC/Bloomberg Events
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │   EVALUATE   │ ◄─── Policy Engine matches rules, sentiment, and drift thresholds
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │  CONSTRAIN   │ ◄─── Risk Engine evaluates concentration caps, LTV, single-trade caps
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │    DECIDE    │ ◄─── Decision Engine generates ExecutionRequest & signs trade manifest
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │   EXECUTE    │ ◄─── Quote evaluation / dry-run logging / Solana Anchor settlement
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │OBSERVE AGAIN │ ◄─── Update portfolio accounting, verify invariant post-conditions
  └──────────────┘
```

---

## 5. Implementation Status Matrix

The codebase currently spans Anchor smart contracts, a pure deterministic mathematical crate, an Axum HTTP API with background workers, integration crates, a TypeScript SDK, and a Next.js web application.

| Subsystem / Feature | Status | Location in Codebase | Notes |
|---|---|---|---|
| **Anchor Vault Core (`equity_vault`)** | `[IMPLEMENTED]` | `programs/equity_vault/src/lib.rs` | Program ID `8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH`. Handles `initialize_vault`, `deposit`, `withdraw`, `update_policy`, `emergency_exit`. |
| **Share Accounting (Pro-rata)** | `[IMPLEMENTED]` | `programs/equity_vault/src/instructions/deposit.rs` | Proportional share minting formula: $\frac{\text{amount} \times \text{total\_shares}}{\text{total\_deposits}}$ |
| **Emergency Pause Controls** | `[IMPLEMENTED]` | `programs/equity_vault/src/instructions/emergency_exit.rs` | Authority-signed circuit breaker halts deposits and withdrawals. |
| **On-Chain Position / Loan / Execution State** | `[PARTIALLY IMPLEMENTED]` | `programs/equity_vault/src/state/` | Structs defined (`Position`, `Loan`, `Execution`), but instruction handlers to create/mutate them on-chain are pending. |
| **Deterministic Math & Risk Crate** | `[IMPLEMENTED]` | `crates/shared/src/` | Pure Rust crate with basis points math, LTV formulas, drift detection, and rebalance algorithms. Zero I/O dependencies. |
| **Axum REST API** | `[IMPLEMENTED]` | `apps/api/src/` | Full HTTP routing for vaults, policies, events, quotes, executions, and health monitoring. |
| **Policy Engine** | `[IMPLEMENTED]` | `apps/api/src/engines/policy_engine/` | Rule matching (`EarningsBeat`, `EarningsMiss`, `DriftRebalance`), sentiment scoring, target weight recalculation. |
| **Risk Engine** | `[IMPLEMENTED]` | `apps/api/src/engines/risk_engine/` | Exposure checks (`validate_position_exposure`), single trade limits (10% max), LTV boundaries, stop-loss triggers. |
| **Decision Engine & Key Signer** | `[IMPLEMENTED]` | `apps/api/src/engines/decision_engine/` | Synthesizes `ExecutionRequest`, isolates execution signing keypair, pre-flight validation. |
| **Quote-Only Execution Service** | `[IMPLEMENTED]` | `apps/api/src/services/quote_service.rs` | Fetches Jupiter v6 quotes, calculates price impact, verifies risk limits, logs audit record (`QUOTE_ONLY`). |
| **Policy Worker (Dry-Run Mode)** | `[IMPLEMENTED]` | `apps/api/src/workers/policy_worker.rs` | Background worker consuming events from Redis/Postgres, running policy/risk pipeline, logging decisions (`LOGGED`). |
| **Live On-Chain Trade Settlement** | `[PLANNED / NOT IMPLEMENTED]` | `apps/api/src/workers/policy_worker.rs` | Live on-chain swap submission is deliberately suppressed to guarantee pre-production safety. |
| **Pyth Hermes Price Oracle** | `[IMPLEMENTED]` | `integrations/pyth/` | HTTP client for latest price feeds with real-time decimal normalization and mock testing mode. |
| **Jupiter v6 Swap Client** | `[IMPLEMENTED]` | `integrations/jupiter/` | Quote fetching, price impact parsing, swap transaction building, and mock modes. |
| **Solana RPC & Anchor Service** | `[IMPLEMENTED]` | `integrations/solana/` | RPC client, WebSocket log listener, and Anchor client with read-only safety lock. |
| **TypeScript SDK** | `[IMPLEMENTED]` | `sdk/src/` | Typed clients for vaults, policies, events, executions, portfolio, and credit. |
| **Next.js Web Application** | `[IMPLEMENTED]` | `apps/web/src/` | Dashboard, vault creation, policy builder, risk meter, position table, and interactive replay demo. |
| **Tokenized Equity Provider Integrations (xStocks, Ondo)** | `[PLANNED / NOT IMPLEMENTED]` | `integrations/xstocks/`, `integrations/ondo/` | Directory stubs present; integration currently routed generically via SPL mints. |

---

## 6. Document Map

To understand the complete mechanics of Equity Catalyst, follow this reading path:
1. **[02-product-model.md](./02-product-model.md)**: Explore the end-user product concepts and robo-portfolio mechanics.
2. **[03-domain-model.md](./03-domain-model.md)**: Inspect entity states, state ownership, and mathematical invariants.
3. **[04-system-architecture.md](./04-system-architecture.md)**: Review component boundaries and Mermaid architecture diagrams.
4. **[07-onchain-architecture.md](./07-onchain-architecture.md)** & **[08-offchain-architecture.md](./08-offchain-architecture.md)**: Understand the critical on-chain vs. off-chain divide.
5. **[11-risk-engine.md](./11-risk-engine.md)** through **[14-execution.md](./14-execution.md)**: Dive into the core deterministic engines.
6. **[26-senior-engineering-review.md](./26-senior-engineering-review.md)**: Review the principal architectural evaluation, risks, and roadmap.
