# Equity Catalyst — Engineering & Architecture Documentation

Welcome to the technical engineering documentation for **Equity Catalyst**: a programmable portfolio controller and institutional-grade equity vault system built for tokenized securities on Solana.

---

## Documentation Navigation Index

| Document | Title | Description | Status |
|---|---|---|---|
| [01-overview.md](./01-overview.md) | Executive Overview & Product Vision | Problem statement, value proposition, core thesis, and system boundaries | Complete |
| [02-shariah-design-rules.md](./02-shariah-design-rules.md) | Shariah Design Rules | The 8 Halal-by-Design rules: spot-only, zero-interest, non-leveraged, ownership | Complete |
| [02-product-model.md](./02-product-model.md) | Product Model & Operational Cycle | The programmable spot robo-vault concept, lifecycle, and target personas | Complete |
| [03-domain-model.md](./03-domain-model.md) | Domain Model & Core Entities | Entities: Vault, Policy, Position, Event, Signal, Decision, Spot Risk, Execution, Trade | Complete |
| [04-system-architecture.md](./04-system-architecture.md) | System Architecture | Component boundaries, subsystem topologies, and Mermaid architectural diagrams | Complete |
| [05-data-flow.md](./05-data-flow.md) | Data Flow & Business Pipelines | Event ingestion flow, decision synthesis flow, and transaction execution flow | Complete |
| [06-state-machines.md](./06-state-machines.md) | State Machines & Lifecycles | Vault, Decision, Execution, and Policy lifecycle transitions | Complete |
| [07-onchain-architecture.md](./07-onchain-architecture.md) | On-Chain Anchor Program (`equity_vault`) | Program ID, PDAs, account layouts, instruction sets, CPIs, and security invariants | Complete |
| [08-offchain-architecture.md](./08-offchain-architecture.md) | Off-Chain Application Layer (`apps/api`) | Axum server, asynchronous Tokio workers, Postgres repositories, and Redis queues | Complete |
| [09-security.md](./09-security.md) | Security Architecture & Defense-in-Depth | Trust boundaries, cryptographic signer isolation, integer math, and invariant protections | Complete |
| [10-threat-model.md](./10-threat-model.md) | Comprehensive Threat Model | Threat vectors, attack scenarios, existing mitigations, and gaps | Complete |
| [11-risk-engine.md](./11-risk-engine.md) | Spot Risk Engine Architecture | Concentration limits, minimum cash reserve, single trade size caps, stop-loss triggers | Complete |
| [12-policy-engine.md](./12-policy-engine.md) | Policy Engine & Rule Matching | Rule conditions, drift detection, sentiment triggers, and target weight computation | Complete |
| [13-decision-engine.md](./13-decision-engine.md) | Decision Engine & Signer Isolation | Pipeline coordination, trade order generation, signer key isolation, and pre-flight validation | Complete |
| [14-execution.md](./14-execution.md) | Execution Engine & Routing | Quote-only execution mode, Jupiter routing, dry-run audit logging, and on-chain settlement | Complete |
| [15-api.md](./15-api.md) | REST API Specification | Axum endpoint definitions, request/response models, validation logic, and error codes | Complete |
| [16-testing.md](./16-testing.md) | Testing Strategy & Test Pyramid | Deterministic shared crate tests, Anchor integration suite, API tests, and mock strategies | Complete |
| [17-debugging.md](./17-debugging.md) | Debugging Architecture & Guide | Single-hypothesis debugging methodology, pipeline checkpoints, and trace logs | Complete |
| [18-observability.md](./18-observability.md) | Observability & Structured Tracing | Correlation ID tracing (`event_id`, `policy_id`, `decision_id`, `execution_id`), metrics | Complete |
| [19-integrations.md](./19-integrations.md) | External Integrations | Solana RPC/WebSocket, Pyth Lazer & Hermes oracles, Jupiter v6 Swap API, mock modes | Complete |
| [20-deployment.md](./20-deployment.md) | Deployment Architecture & Environment | Local validator, devnet deployment, PostgreSQL migrations, Docker configurations | Complete |
| [21-performance.md](./21-performance.md) | Performance & Optimization Analysis | Hot paths, memory allocations, async bottlenecks, database index efficiency | Complete |
| [22-adr.md](./22-adr.md) | Architectural Decision Records (ADRs) | ADR-001 through ADR-006 detailing core architectural trade-offs and rationale | Complete |
| [23-failure-modes.md](./23-failure-modes.md) | Failure Modes & Recovery Matrix | Stale oracles, RPC timeouts, slippage breaches, transaction rejection mitigations | Complete |
| [24-repository-map.md](./24-repository-map.md) | Complete Repository File Map | Exhaustive directory tree of the repository with component descriptions | Complete |
| [25-current-vs-target.md](./25-current-vs-target.md) | Current State vs. Target State | Gap analysis categorized by priority (P0 to P3) | Complete |
| [26-senior-engineering-review.md](./26-senior-engineering-review.md) | Senior Engineering Review & Roadmap | What is good, what is weak, what is risky, what is over-engineered, and top 10 improvements | Complete |

---

## Shariah Supervisory Board Review Dossier

Detailed Shariah compliance documentation is located in [`docs/shariah-review/`](./shariah-review/):

| Document | Title | Description | Standard |
|---|---|---|---|
| [asset-structure.md](./shariah-review/asset-structure.md) | Asset Structure & Representation | 1:1 fractional ownership (*Musha'*), custodial segregation, AAOIFI screening | AAOIFI Std 21 |
| [prohibited-features.md](./shariah-review/prohibited-features.md) | Permanently Eliminated Prohibitions | Purge of loans, borrowing, margin leverage, short selling, and synthetics | AAOIFI / IIFA |
| [ownership-model.md](./shariah-review/ownership-model.md) | Ownership & Constructive Possession | *Milkiyyah Taammah*, *Qabd Hukmi*, Solana finality, dividend pass-through | AAOIFI Std 18 |
| [transaction-flow.md](./shariah-review/transaction-flow.md) | Spot Transaction Flow | Simultaneous exchange (*Taqaabud*), $T+0$ spot settlement, Pyth Lazer bounds | *Bay' al-Hadir* |
| [fee-structure.md](./shariah-review/fee-structure.md) | Transparent Fee Economics | Unbundled 15 bps fee model, zero interest spreads, and absence of liquidation fees | *Ujrah* Rules |
| [questions-for-review.md](./shariah-review/questions-for-review.md) | Questions for Shariah Boards | 5 structured supervisory questions on possession, bonding curves, purification | Board Review |

---

## Implementation Status Legend

Throughout this documentation, system elements and architectural modules are strictly annotated using the following taxonomy:

* **`[IMPLEMENTED]`**: Fully coded, tested with unit or integration tests, and operational in the current codebase.
* **`[PARTIALLY IMPLEMENTED]`**: Scaffolded or implemented with constrained functionality (e.g. dry-run logging only, or client-side instruction builder existing without on-chain Anchor handler).
* **`[PLANNED / NOT IMPLEMENTED]`**: Defined in architecture designs, folder structures, or database schemas, but lacking active executable code.
* **`[RECOMMENDED]`**: Senior architect recommendation for production hardening, refactoring, or security enhancement.

---

## Core Operational Cycle

```text
       ┌───────────────┐
       │ External Event│ (Pyth Lazer 200ms, Hermes, Earnings, DBC State)
       └───────┬───────┘
               │
               ▼
       ┌───────────────┐
       │ Policy Engine │ (Shariah Screening, Sentiment Scoring, Target Allocation)
       └───────┬───────┘
               │
               ▼
       ┌───────────────┐
       │Spot Risk Engine│ (Exposure Limits, Minimum Cash Reserve, Single Trade Caps)
       └───────┬───────┘
               │
               ▼
       ┌───────────────┐
       │Decision Engine│ (Order Synthesis, Key Isolation, 5-Stage Gate Validation)
       └───────┬───────┘
               │
               ▼
       ┌───────────────┐
       │Execution Layer│ (Meteora DBC Spot Quotes, Atomic Settlement, Reconciliation)
       └───────┬───────┘
               │
               ▼
       ┌───────────────┐
       │   Observer    │ (WebSocket State Mirroring, DB Accounting Sync, Ujrah Audit)
       └───────────────┘
```
