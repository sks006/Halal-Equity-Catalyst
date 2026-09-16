# ADR-005: Agent Safety & Dual-Line Risk Engine

## Status
Accepted

## Context
Autonomous AI agents provide intelligent macro synthesis, sentiment analysis, and dynamic portfolio rebalancing proposals. However, giving an LLM direct signing authority over user funds creates severe security risks (jailbreaking, prompt injection, hallucinated transactions).

## Decision
We enforce a strict **Dual-Line Risk Architecture**:
1. **Agent Role**: LLM / algorithmic agents may synthesize events and generate *execution proposals* (`ProposedAction`).
2. **First Line of Defense (Off-Chain Risk Engine)**:
   - Evaluates portfolio exposure, maximum LTV, single-asset allocation ceilings, and stop-loss criteria.
   - Emits a deterministic `APPROVE` or `REJECT` decision.
3. **Key Isolation (Decision Engine)**:
   - Only approved proposals pass to the isolated `DecisionEngine` signer.
4. **Second Line of Defense (On-Chain Anchor Program)**:
   - The on-chain `equity_vault` program independently validates all arithmetic, LTV ceilings, and signer authorizations. Even if the off-chain risk engine were compromised, the smart contract prevents illicit extraction.

## Consequences
### Positive
- Immune to prompt injection attacks or LLM hallucinations leading to financial loss.
- Full auditable paper trail of every proposal, risk evaluation, and execution in PostgreSQL.

### Negative
- Higher latency for trade execution due to multi-tier validation pipeline.
