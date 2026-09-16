# Architecture

## Product

Equity Catalyst is an agentic liquidity and price-discovery system
for tokenized/pre-IPO assets.

## System flow

```
Asset Provider
      ↓
Market Data
      ↓
Equity Market Engine
      ↓
DBC Configuration Engine
      ↓
Meteora DBC
      ↓
Monitoring
      ↓
Risk Engine
      ↓
Agent Decision
      ↓
Execution
      ↓
Graduation
      ↓
DAMM v2
```

## Major systems

### apps/api

Rust backend.

Responsibilities:

- HTTP API
- WebSocket handling
- Pyth integration
- asset providers
- market calculations
- DBC orchestration
- risk engine
- agent
- workers
- persistence
- execution

### apps/web

Next.js frontend.

Responsibilities:

- dashboard
- market comparison
- DBC configuration
- pool monitoring
- agent visualization
- transaction status

### crates/shared

Pure deterministic domain logic.

Allowed:

- pricing math
- premium calculations
- risk calculations
- curve calculations
- graduation calculations
- domain types

Not allowed:

- Axum
- Tokio tasks
- HTTP clients
- SQL
- secrets
- wallet signing

### programs/equity_vault

Anchor program.

Responsibilities:

- vault ownership
- authority
- policy
- risk invariants
- execution authorization
- accounting
- emergency controls

It does NOT reproduce Meteora DBC internals.

### integrations

External protocol/provider adapters.
