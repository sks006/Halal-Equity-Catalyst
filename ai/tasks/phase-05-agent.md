# Phase 05 — Liquidity Agent

Status: DONE
Owner: Agent 5
Dependencies: Phase 04
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [x] verification
- [x] documentation
- [x] review

## Objective

Create a deterministic agent that observes the market and proposes bounded actions.

## Context

The Liquidity Agent acts as the intelligent observing eye of the vault and liquidity pools. It consumes synthesized market intelligence and health signals from the Equity Engine, matches them against vault policy rules, and submits proposed actions to the Risk Engine. The agent is strictly bounded and cannot directly sign or authorize transactions.

## Flow

```
Pyth
  ↓
DBC
  ↓
Market Engine
  ↓
Equity Health
  ↓
Policy
  ↓
Risk
  ↓
Decision
```

## Allowed Files

- `apps/api/src/agents/liquidity_agent/**`
- `apps/api/src/agents/mod.rs`
- `apps/api/src/engines/decision_engine/**`
- `crates/shared/src/models/agent.rs`
- `tests/agent/**`

## Forbidden Files

- `apps/api/src/engines/decision_engine/signer.rs` (agent must have zero access to signer or keypairs)
- Direct transaction submission methods (`SolanaService::send_transaction`)
- Frontend browser bundles (`apps/web/**`)

## Requirements

1. **Deterministic Evaluation**: Given identical market feeds, health status, and vault state, the agent must produce the identical decision every time.
2. **Allowed Decisions**:
   - `HOLD`: Market is healthy; existing liquidity and portfolio allocations are optimal.
   - `MONITOR`: Elevated volatility or minor premium; increase observation frequency without taking action.
   - `REDUCE_EXPOSURE`: Market health warning or approaching stop-loss limit; propose trimming position.
   - `HALT_AUTOMATION`: Critical oracle failure, excessive premium decoupling ($> 500\text{ bps}$), or circuit breaker triggered.
   - `GRADUATION_READY`: DBC curve threshold achieved; recommend initiating migration to Meteora DAMM v2.
3. **Strict Invariants**:
   - Zero direct private-key access.
   - Zero direct transaction authorization.
   - Never bypass the Risk Engine.

## Implementation Steps

1. Define `AgentDecision` enum and `ProposedAction` struct in `crates/shared/src/models/agent.rs`.
2. Build `LiquidityAgent` in `apps/api/src/agents/liquidity_agent/`.
3. Ingest `EquityHealth` evaluation, vault allocations, and active policy parameters.
4. Implement rule matching logic emitting one of the 5 allowed decisions.
5. Connect agent proposal pipeline to the `RiskEngine` for validation.
6. Write reproducibility unit tests verifying decision determinism across varied market conditions.

## Tests

- Reproducibility test: identical market state evaluated $1,000\times$ yields identical decision.
- Critical health state $\implies$ emits `HALT_AUTOMATION`.
- Curve progress $\ge 100\%$ $\implies$ emits `GRADUATION_READY`.
- Moderate decoupling ($300\text{ bps}$) with high LTV $\implies$ emits `REDUCE_EXPOSURE`.
- Normal spread and healthy oracle $\implies$ emits `HOLD`.

## Verification Commands

```bash
cargo test --package equity-catalyst-api liquidity_agent
cargo test --package equity-catalyst-api reproducibility
```

## Acceptance Criteria

- Agent decision is $100\%$ reproducible for identical inputs.
- Agent output cannot bypass the Risk Engine or trigger execution without approval.
- Agent code contains zero references to private keys, keypairs, or transaction signing.

## Documentation Updates

- Update `ai/domain.md` with Agent bounded decision definitions.
- Update `ai/current-state.md` upon completion.

## Failure Conditions

- Non-deterministic decision output (e.g. relying on unseeded RNG or system clocks in core logic).
- Direct invocation of transaction signing or submission from agent modules.
- Bypassing the Policy or Risk Engine.
