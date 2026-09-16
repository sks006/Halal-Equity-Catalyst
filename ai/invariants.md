# System Invariants

## Security

1. Pyth Pro API key is server-side only.
2. Private signing material never enters browser code.
3. Agent cannot bypass the risk engine.
4. Risk rejection must stop execution.
5. Unknown assets cannot be automatically traded.
6. Unknown quote tokens cannot be automatically used.

## Market-data safety

1. Every price has a source.
2. Every price has a timestamp.
3. Stale data cannot authorize automated execution.
4. Reference-price calculations require a valid denominator.
5. Provider failure must fail closed for automated execution.

## DBC

1. DBC configuration must pass Meteora-supported constraints.
2. Never assume undocumented SDK behavior.
3. Pool address/config address/mints must be persisted.
4. Mainnet transactions must be independently verifiable.

## Execution

1. Every execution has an idempotency key.
2. Transaction parameters are validated before signing.
3. Execution results are persisted.
4. Failed transactions cannot silently become successful.
5. Retry logic cannot duplicate an execution.

## Agent

1. Agent proposes actions.
2. Policy determines whether proposal is allowed.
3. Risk engine validates proposed action.
4. Only validated actions can reach execution.
5. LLM output is never authoritative financial authorization.

## Anchor

1. On-chain invariants are authoritative.
2. API-side validation does not replace on-chain validation.
3. PDA relationships must be validated on-chain.
4. Emergency controls must remain available.
