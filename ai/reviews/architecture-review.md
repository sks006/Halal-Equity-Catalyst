# Architectural Review

## Evaluation Summary
- **Decoupling**: Strong boundary between off-chain calculation (`apps/api`) and on-chain state custody (`programs/equity_vault`). Smart contracts are not burdened with external HTTP calls.
- **Data Flow**: Unidirectional pipeline from on-chain WebSocket events → Redis → Policy Worker → Risk Engine → Decision Engine.
- **State Management**: PostgreSQL is used strictly as the relational audit log and indexed query cache; on-chain Solana state remains the single source of truth for assets and vault equity.
- **Extensibility**: Provider adapters allow seamless integration of new oracle feeds or DEX routing protocols without modifying business logic.

## Findings & Recommendations
1. Ensure the PostgreSQL connection pool (`deadpool-postgres`) has appropriate connection limits and timeouts configured for burst traffic.
2. Maintain strict separation between UI presentation state (Redux) and cryptographic transaction submission.
