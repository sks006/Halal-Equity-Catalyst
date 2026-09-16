# ADR-004: Provider Adapter Pattern for External Protocol Integrations

## Status
Accepted

## Context
Equity Catalyst interfaces with diverse external protocols: Pyth, Jupiter, Meteora, PreStocks, and Tessera. Embedding protocol-specific HTTP schemas or IDL details directly into core domain services causes tight coupling, fragile testing, and breaking changes whenever an upstream API changes.

## Decision
We implement a strict **Provider Adapter Pattern**:
- All external protocols are isolated in dedicated integration modules under `integrations/` (or `sdk/src/meteora/`).
- External wire types are deserialized in the adapter and translated into normalized internal domain types (`crates/shared`).
- Domain engines (Policy, Risk, Decision) only interact with internal trait abstractions.

## Consequences
### Positive
- External API changes (e.g., Jupiter v6 to v7) affect only a single adapter file.
- Domain engines can be unit-tested using deterministic mocks without network access.

### Negative
- Requires maintaining translation boilerplate between external models and internal domain types.
