# Equity Catalyst

> **Institutional-Grade Programmable Portfolio Controller & Autonomous Equity Vault on Solana**

[![Solana](https://img.shields.io/badge/Solana-Anchor%20v0.30.1-14F195?logo=solana)](https://solana.com)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-DEA584?logo=rust)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-3178C6?logo=typescript)](https://www.typescriptlang.org)
[![Next.js](https://img.shields.io/badge/Next.js-14%2B-000000?logo=next.js)](https://nextjs.org)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

---

## Executive Overview

**Equity Catalyst** makes tokenized equities programmable on Solana. Rather than treating tokenized stocks (e.g. `NVDAx`, `MSFTx`, `AAPLx`) as static, buy-and-hold assets in personal wallets, Equity Catalyst allows investors and asset managers to wrap them into non-custodial, policy-governed smart vaults that dynamically rebalance, hedge, and manage risk in response to real-world financial events.

The platform focuses strictly on the **policy and control plane**:
* **Observe**: Ingest real-time market data, Pyth oracle feeds, corporate earnings surprises, and macro indicators.
* **Evaluate**: Match policy rules, compute portfolio drift, and calculate target asset allocations.
* **Constrain**: Pass all proposed rebalance orders through a multi-factor risk defense engine (position exposure caps, 10% single-trade limits, LTV ceilings, stop-loss triggers).
* **Decide**: Synthesize immutable execution manifests signed by an isolated execution keeper.
* **Execute**: Route swaps through Jupiter v6 DEX aggregation with price-impact and slippage validation.
* **Observe Again**: Mirror on-chain state to PostgreSQL and verify accounting invariants.

---

## Architectural Topology

```mermaid
graph TB
    subgraph ClientLayer [Presentation & SDK Layer]
        Web[Next.js 14 Web App<br/>apps/web]
        SDK[TypeScript Client SDK<br/>sdk/src]
        Demo[Deterministic Replay Script<br/>scripts/replay-demo.ts]
    end

    subgraph APILayer [Application & Service Layer: apps/api]
        Router[Axum HTTP Router<br/>Port 8080]
        QuoteSvc[QuoteExecutionService<br/>Jupiter v6 Quote Evaluation]
        SolanaSvc[SolanaService<br/>RPC & WS Coordination]
        OracleSvc[OracleService<br/>Pyth Hermes Normalization]
    end

    subgraph WorkersLayer [Asynchronous Background Workers]
        EvtListener[EventListener Worker<br/>Redis Queue Pusher]
        PolWorker[PolicyWorker<br/>Consumer & Dry-Run Auditor]
    end

    subgraph EngineLayer [Deterministic Domain Engines]
        PE[Policy Engine<br/>rules, signals, allocation]
        RE[Risk Engine<br/>exposure, limits, ltv, stops]
        DE[Decision Engine<br/>manifest synthesis & isolated signer]
    end

    subgraph SharedLayer [Pure Deterministic Domain Crate]
        SharedMath[crates/shared<br/>pure math, basis points, drift, LTV, risk]
    end

    subgraph InfraLayer [Persistence & External Integrations]
        PG[(PostgreSQL Database<br/>vaults, policies, events, executions)]
        Redis[(Redis FIFO Queue<br/>events:queue)]
        PythInt[Pyth Hermes Oracle<br/>integrations/pyth]
        JupInt[Jupiter v6 DEX Aggregator<br/>integrations/jupiter]
        SolInt[Solana Cluster<br/>integrations/solana]
    end

    subgraph OnChainLayer [Solana Anchor Program: programs/equity_vault]
        VaultProg[Anchor Program: equity_vault<br/>Program ID: 8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH]
    end

    Web --> SDK
    Demo --> SDK
    SDK --> Router
    SDK --> VaultProg

    Router --> QuoteSvc
    Router --> SolanaSvc
    Router --> OracleSvc
    Router --> PG

    EvtListener --> Redis
    EvtListener --> PG

    PolWorker --> Redis
    PolWorker --> PG
    PolWorker --> DE

    DE --> PE
    DE --> RE
    PE --> SharedMath
    RE --> SharedMath

    QuoteSvc --> JupInt
    QuoteSvc --> RE
    OracleSvc --> PythInt
    SolanaSvc --> SolInt
    SolanaSvc --> VaultProg
```

---

## Core Engineering Documents Index

The repository features comprehensive, staff-level technical documentation in the [`docs/`](./docs/) directory:

| Document | Description |
|---|---|
| [01. Executive Overview](./docs/01-overview.md) | Problem statement, value proposition, core thesis, and system boundaries |
| [02. Product Model](./docs/02-product-model.md) | The programmable robo-vault concept, lifecycle, and target personas |
| [03. Domain Model](./docs/03-domain-model.md) | Detailed entity specs and $10,000 NVDA earnings surprise trace |
| [04. System Architecture](./docs/04-system-architecture.md) | Component boundaries, subsystem topologies, and Mermaid architecture diagrams |
| [05. Data Flow](./docs/05-data-flow.md) | Ingestion flow, decision synthesis flow, and transaction execution flow |
| [06. State Machines](./docs/06-state-machines.md) | Vault, Decision, Execution, Policy, and Loan lifecycle state machines |
| [07. On-Chain Architecture](./docs/07-onchain-architecture.md) | Anchor program ID `8NhtqxR1...`, PDAs, account layouts, and instructions |
| [08. Off-Chain Architecture](./docs/08-offchain-architecture.md) | Axum server, asynchronous Tokio workers, Postgres repositories, and Redis queues |
| [09. Security Architecture](./docs/09-security.md) | Trust boundaries, cryptographic signer isolation, integer math, and invariant protections |
| [10. Threat Model](./docs/10-threat-model.md) | STRIDE threat matrix, attack scenarios, existing mitigations, and gaps |
| [11. Risk Engine](./docs/11-risk-engine.md) | Concentration limits, LTV boundaries, single trade caps, and stop-loss triggers |
| [12. Policy Engine](./docs/12-policy-engine.md) | Rule conditions, drift detection, sentiment triggers, and target weight computation |
| [13. Decision Engine](./docs/13-decision-engine.md) | Pipeline coordination, order manifest generation, and pre-flight checks |
| [14. Execution Engine](./docs/14-execution.md) | Quote-only execution mode, Jupiter routing, and dry-run audit logging |
| [15. REST API Specification](./docs/15-api.md) | Axum endpoint definitions, request/response models, validation logic, and error codes |
| [16. Testing Strategy](./docs/16-testing.md) | Test pyramid: unit tests in shared crate, Anchor integration suite, and replay demo |
| [17. Debugging Architecture](./docs/17-debugging.md) | Single-hypothesis debugging methodology, pipeline checkpoints, and trace logs |
| [18. Observability](./docs/18-observability.md) | Structured logging around correlation IDs (`event_id`, `vault_address`, `decision_id`) |
| [19. External Integrations](./docs/19-integrations.md) | Solana RPC/WebSocket, Pyth Hermes HTTP oracle, Jupiter v6 Swap API, mock modes |
| [20. Deployment Architecture](./docs/20-deployment.md) | Local validator, devnet deployment, PostgreSQL migrations, Docker configurations |
| [21. Performance & Efficiency](./docs/21-performance.md) | Hot paths, memory allocations, async bottlenecks, database index efficiency |
| [22. Architectural Decisions (ADRs)](./docs/22-adr.md) | ADR-001 through ADR-006 detailing core architectural trade-offs and rationale |
| [23. Failure Scenarios & Recovery](./docs/23-failure-modes.md) | Stale oracles, RPC timeouts, slippage breaches, transaction rejection mitigations |
| [24. Repository File Map](./docs/24-repository-map.md) | Complete directory tree of the repository with component descriptions |
| [25. Current State vs Target State](./docs/25-current-vs-target.md) | Gap analysis categorized by priority (P0 to P3) |
| [26. Senior Engineering Review](./docs/26-senior-engineering-review.md) | Principal engineer review: what is good, weak, risky, over-engineered, and top 10 roadmap |

---

## Quickstart & Local Development

### 1. Prerequisites
* **Rust**: `v1.75.0` or higher (`rustc --version`)
* **Solana CLI**: `v1.18.26` or higher (`solana --version`)
* **Anchor**: `v0.30.1` (`anchor --version`)
* **Node.js**: `v18+` or `v20+` and `pnpm`
* **Docker**: Docker & Docker Compose (for PostgreSQL and Redis)

### 2. Start PostgreSQL & Redis
```bash
docker run -d --name equity-postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=equity_catalyst \
  -p 5432:5432 postgres:16-alpine

docker run -d --name equity-redis \
  -p 6379:6379 redis:7-alpine
```

### 3. Run Database Migrations
```bash
for file in db/migrations/*.sql; do
  psql postgres://postgres:postgres@localhost:5432/equity_catalyst -f "$file"
done
```

### 4. Run the Deterministic Demo Replay
Experience the full 7-stage event-to-execution pipeline without needing a live testnet wallet:
```bash
npx ts-node scripts/replay-demo.ts --fast
```

### 5. Build and Test
```bash
# Run unit tests in pure deterministic domain crate (12 passing tests)
cargo test -p equity-catalyst-shared

# Build on-chain Anchor smart contract
anchor build

# Run Anchor integration tests on local validator (15 passing tests)
anchor test --skip-build

# Start Axum HTTP backend API
cargo run -p equity-catalyst-api

# Start Next.js web application
pnpm --filter web install
pnpm --filter web dev
```

---

## On-Chain Program Details

* **Program Name**: `equity_vault`
* **Program ID**: `8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH`
* **On-Chain Instructions**:
  1. `initialize_vault`: Initializes Vault PDA and Policy PDA with authority and risk bounds.
  2. `deposit`: Transfers underlying SPL tokens and mints pro-rata LP shares ($\lfloor \frac{\text{amount} \times S_{\text{total}}}{D_{\text{total}}} \rfloor$).
  3. `withdraw`: Burns LP shares and transfers pro-rata underlying assets ($\lfloor \frac{\text{shares} \times D_{\text{total}}}{S_{\text{total}}} \rfloor$).
  4. `update_policy`: Updates max LTV, concentration limits, and stop-loss parameters (Authority only).
  5. `emergency_exit`: Authority-triggered circuit breaker toggling `is_paused = true`.

---

## License

Apache-2.0 License. See [LICENSE](LICENSE) for details.
