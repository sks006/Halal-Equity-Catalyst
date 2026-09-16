# Phase 03 — Meteora DBC

Status: DONE
Owner: Agent 3
Dependencies: Phase 02
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [x] verification
- [x] documentation
- [x] review

## Objective

Create and inspect a real stock-paired DBC pool.

## Context

Integrate the official Meteora Dynamic Bonding Curve (DBC) SDK to establish customized price-discovery curves tailored for tokenized equity assets (e.g. Backed NVIDIA - `NVDAx`), derive deterministic pool PDAs, inspect on-chain reserves, execute swaps, and persist launch records in PostgreSQL.

## Allowed Files

- `sdk/src/meteora/**`
- `integrations/meteora/**`
- `apps/api/src/engines/dbc_engine/**`
- `apps/api/src/services/meteora_service.rs`
- `apps/api/src/models/dbc_pool.rs`
- `apps/api/src/repositories/dbc_pool_repository.rs`
- `apps/api/src/routes/dbc.rs`
- `scripts/launch-real-pool.ts`
- `scripts/simulate-equity-curve.ts`
- `tests/meteora/**`

## Forbidden Files

- Manually written Anchor instruction deserializers that bypass the official SDK
- Invented binary account layouts
- Direct modifications to core Anchor programs outside vault interfaces

## Requirements

1. **Pin Current Meteora SDK**: Pin `@meteora-ag/dynamic-bonding-curve-sdk` (v1.5.12).
2. **DBC Config Builder**: Implement `buildEquityDiscoveryCurve` generating Meteora `ConfigParameters` with 3 piecewise regimes and dynamic anti-snipe linear fee scheduler.
3. **Pool Creation**: Implement atomic or bundled pool creation using `client.partner.createConfigAndPool`.
4. **State Reads**: Read pool state, base/quote reserves, and Q64.64 sqrtPrice via `client.state.getPool`.
5. **Quote / Swap Support**: Support quote and swap transaction formulation via `client.pool.swap`.
6. **Migration State**: Track DAMM v2 migration threshold and curve completion percentage.
7. **Pool Persistence**: Persist pool address, config address, base mint, quote mint, transaction signature, and creation timestamp into PostgreSQL `dbc_pools`.
8. **Rules**:
   - Use current official SDK methods (`buildCurveWithCustomSqrtPrices`, `createSqrtPrices`, `deriveDbcPoolAddress`).
   - Never follow deprecated tutorials or invent account structures.
   - All transactions must be independently verifiable on Solana.

## Implementation Steps

1. Configure `@meteora-ag/dynamic-bonding-curve-sdk` dependency in `sdk/package.json`.
2. Build `buildEquityDiscoveryCurve` compiling 3 price checkpoints and liquidity weights.
3. Build `MeteoraDbcService` in `sdk/src/meteora/service.ts` exposing config/pool creation and pool summary reads.
4. Implement PostgreSQL schema migration `007_dbc_pools.sql` and API model/repository.
5. Implement backend route `POST /dbc/pools` and `GET /dbc/pools`.
6. Implement operational script `scripts/launch-real-pool.ts` to execute creation of real `NVDAx`/USDC pool.
7. Confirm persistence of addresses and transaction signatures.

## Tests

- Monotonic sqrtPrice progression validation.
- Deterministic PDA derivation (`deriveDbcPoolAddress`, `deriveDbcTokenVaultAddress`).
- Swap quote calculation and slippage checks.
- Migration threshold calculation.
- Database insert and query idempotency.

## Verification Commands

```bash
npx ts-node scripts/simulate-equity-curve.ts
npx ts-node scripts/launch-real-pool.ts
docker exec equity-postgres psql -U postgres -d equity_catalyst -c "SELECT pool_address, token_symbol, base_mint FROM dbc_pools;"
curl -s http://localhost:8080/dbc/pools | jq .
```

## Acceptance Criteria

- One real stock/USDC pool successfully launched and persisted in PostgreSQL.
- Database contains:
  - `config address`
  - `pool address`
  - `base mint` (e.g. `Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh`)
  - `quote mint` (USDC)
  - `creation signature`

## Documentation Updates

- Update `ai/current-state.md` with launched pool details.
- Record Meteora DBC program IDs and PDA derivation notes in `ai/context/meteora.md`.

## Failure Conditions

- Non-monotonic price progression ($P_0 \ge P_1$).
- Failure to persist pool or config addresses to PostgreSQL.
- Relying on reverse-engineered raw instruction layouts instead of the official SDK.
