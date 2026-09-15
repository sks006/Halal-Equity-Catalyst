# 22. Architectural Decision Records (ADRs)

## ADR-001: Programmable Portfolio Controller vs. Building Another Exchange

### Context
Tokenized equities are emerging on Solana. A common impulse in DeFi is to launch a dedicated Central Limit Order Book (CLOB), Automated Market Maker (AMM), or synthetic derivatives exchange for equities.

### Decision
Equity Catalyst focuses exclusively on the **programmable portfolio controller / autonomous vault layer**, completely eschewing the creation of a new exchange.

### Alternatives Considered
* Building a proprietary CLOB for tokenized stocks.
* Issuing synthetic equity perpetual futures.

### Reasoning
Solana already has mature, hyper-optimized DEX infrastructure (Jupiter, Phoenix, OpenBook, Raydium). Rebuilding liquidity pools splits market depth and requires massive market-making incentives. The real unaddressed friction is that existing tokenized equities are static buy-and-hold assets without automated management, rebalancing, or risk guardrails.

### Consequences
* **Positive**: Rapid time-to-market, instant access to deep Solana liquidity, focused domain model.
* **Negative**: Dependency on external DEX liquidity depth for tokenized stock pairs.

---

## ADR-002: Off-Chain Policy Evaluation

### Context
Policy evaluation involves processing corporate earnings surprises, sentiment scores, multi-asset drift metrics, and complex rebalancing algorithms.

### Decision
Execute policy evaluation, rule matching, and signal generation in an off-chain application layer (`apps/api`), passing validated trade manifests to Solana.

### Alternatives Considered
* Implementing an on-chain virtual machine or full rule engine inside Anchor smart contracts.

### Reasoning
1. Floating point math, complex string parsing (SEC filings), and external API communication cannot run on-chain.
2. Solana's $1.4\text{M}$ Compute Unit limit would be exhausted by complex multi-asset optimization.
3. Proprietary trading desks require privacy for their strategy logic.

### Consequences
* **Positive**: Unconstrained computational expressiveness, sub-second execution speed, privacy for trading alpha.
* **Negative**: Requires robust off-chain backend infrastructure and independent verification of integrity.

---

## ADR-003: On-Chain Enforcement of Security-Critical Invariants

### Context
If policy evaluation occurs off-chain, what prevents a compromised server, malicious keeper, or buggy algorithm from draining user funds or taking catastrophic leverage?

### Decision
Enforce all critical financial boundaries directly in Anchor contract bytecode:
1. Hard position exposure caps (`max_position_bps`).
2. Maximum borrowing boundaries (`max_ltv_bps`).
3. Non-custodial share accounting (pro-rata share minting and burning formulas).
4. Authority-only emergency pause controls.

### Alternatives Considered
* Pure off-chain trust: Relying entirely on server-side risk checks and giving the backend unrestricted custody keys.

### Reasoning
In DeFi, code is law. Users should never trust an off-chain API with their life savings. By encoding hard mathematical ceilings into the smart contract, even a total RCE compromise of the API server cannot violate investor risk limits.

### Consequences
* **Positive**: Institutional-grade safety, non-custodial trust guarantee, tamper-proof risk limits.
* **Negative**: Requires careful account derivation and dual-layer parameter synchronization.

---

## ADR-004: Composing Existing Solana Liquidity (Jupiter v6)

### Context
Rebalancing equity vaults requires executing cross-token swaps (e.g. USDC to NVDAx).

### Decision
Route all rebalancing trades through Jupiter's v6 aggregation API and swap protocols.

### Alternatives Considered
* Direct integration with individual Raydium or Orca pools.
* Internal P2P order matching between vaults.

### Reasoning
Jupiter aggregates virtually all on-chain Solana liquidity across AMMs and order books, automatically discovering multi-hop routes and minimizing slippage.

### Consequences
* **Positive**: Optimal execution rates, access to diverse liquidity sources, minimal custom routing code.
* **Negative**: Dependency on Jupiter API availability (mitigated via local quote validation and mock fallbacks).

---

## ADR-005: Decoupled Five-Engine Architecture (Event, Policy, Risk, Decision, Execution)

### Context
Algorithmic trading systems often suffer from "spaghetti code" where data fetching, trade generation, risk checks, and blockchain RPC calls are mixed in single monolithic functions.

### Decision
Enforce strict separation between:
1. `Event`: Normalized market stimulus.
2. `Policy`: Evaluates rules and desires.
3. `Risk`: Enforces boundaries and safety (can veto Policy).
4. `Decision`: Coordinates and cryptographically signs manifests.
5. `Execution`: Dispatches, evaluates quotes, and audits.

### Alternatives Considered
* A single `StrategyService` executing monolithic trade loops.

### Reasoning
Separation of concerns allows independent unit testing of math (in `crates/shared`), guarantees that the Risk Engine has unconditional veto power, and ensures that execution can be dry-run audited without affecting policy logic.

### Consequences
* **Positive**: Maximum testability, clear fault isolation, deterministic debugging.
* **Negative**: Slightly higher initial code boilerplate.

---

## ADR-006: Modular Monolith vs. Microservices Architecture

### Context
Should the off-chain system be split into 6 separate microservices (Event Service, Policy Service, Risk Service, Execution Service, API Gateway, Solana Worker)?

### Decision
Build the off-chain backend as a **Modular Rust Monolith** (`apps/api`) with asynchronous background worker threads running in the same Tokio process.

### Alternatives Considered
* Deploying separate Docker microservices communicating via gRPC or Kafka.

### Reasoning
Microservices introduce severe operational complexity: distributed transactions, network latency between engines, serialization overhead, and fragmented logging. A modular Rust monolith provides compile-time type safety, zero-latency in-memory function calls, and single-binary deployment while retaining strict module boundaries.

### Consequences
* **Positive**: Simple deployment, unified tracing with single correlation IDs, blazing fast in-memory execution.
* **Negative**: Scaled horizontally as a unified service rather than scaling individual engines independently.
