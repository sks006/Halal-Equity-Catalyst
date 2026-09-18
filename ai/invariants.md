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

## Hardening & Operations (Phase 09)

1. Administrative routes require constant-time API key verification (`x-admin-key` or `Authorization: Bearer`).
2. Public and administrative endpoints are protected by bounded sliding-window rate limiters; exhaustions reject with HTTP 429.
3. Pre-signing emergency pause recheck: Vault pause and policy deactivation abort execution immediately before Step 5 (signing boundary).
4. Secret redaction: Secrets wrapped in `RedactedSecret<T>` and custom `Debug` implementations display `[REDACTED]` in logs, errors, and traces.
5. API errors sanitize internal connection strings, database credentials, and authorization headers before returning HTTP responses.
6. Solana RPC retry differentiates permanent client errors (400, 401, 403, 404, which fail immediately) from transient network errors (429, 502, 503, 504, retried with bounded backoff and jitter).
7. Worker supervisor catches panic unwinds via `tokio::spawn`, applies exponential backoff, and tracks restart windows up to a configurable threshold.
8. Dead-letter queue captures all unprocessable, malformed, or rejected pipeline events in PostgreSQL for post-mortem auditing.
9. Execution idempotency encompasses all operational states (`requested`, `simulated`, `submitted`, `confirmed`) and handles concurrent insertion collisions safely.
