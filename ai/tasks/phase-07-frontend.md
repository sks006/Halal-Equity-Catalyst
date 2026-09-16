# Phase 07 — Frontend

Status: DONE
Owner: Agent 7
Dependencies: Phase 06
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [x] verification
- [x] documentation
- [x] review

## Objective

Build the user-facing web dashboard and visual interaction layer.

## Context

The web frontend provides institutional operators and retail participants with real-time visibility into market intelligence, bonding curve dynamics, pool creation, and autonomous agent decisions. It is built with Next.js 14 App Router, Tailwind CSS, Shadcn UI primitives, and React-Redux.

## Screens & Routes

1. **`/markets`**:
   - Reference price (Pyth Pro)
   - Tokenized price (on-chain / DBC)
   - Premium / discount in basis points
   - Oracle freshness and confidence status
2. **`/launch`**:
   - Asset selector (verified canonical stocks, e.g. `NVDAx`, `AAPLx`)
   - Quote token selector (USDC, SOL)
   - Piecewise 3-regime curve configuration (floor, parity, cap)
   - DAMM v2 graduation configuration
   - Real-time interactive curve preview and PDA derivation
3. **`/pool/[address]`**:
   - Live DBC price and Pyth reference price
   - Premium / spread gauge
   - Real-time liquidity reserves (base / quote)
   - Interactive bonding curve chart with current progress point
   - Equity Health status badge (`HEALTHY`, `WARNING`, `CRITICAL`)
   - Current Agent Decision badge (`HOLD`, `MONITOR`, `REDUCE_EXPOSURE`, `HALT`, `GRADUATION_READY`)
   - Graduation progress bar and migration threshold
4. **`/demo`**:
   - Interactive simulator demonstrating market shocks, premium decoupling, and autonomous agent response.

## Allowed Files

- `apps/web/src/app/**`
- `apps/web/src/components/**`
- `apps/web/src/features/**`
- `apps/web/src/store/**`
- `apps/web/src/lib/**`
- `apps/web/tailwind.config.ts`

## Forbidden Files

- Any storage of `PRO_API_KEY`, RPC private keys, or wallet seed phrases
- Direct database connections or SQL queries from frontend code
- Direct signing bypass outside standard Solana Wallet Adapter interfaces

## Requirements

1. Pure white aesthetic (`#FFFFFF` background, slate high-contrast typography, border accents).
2. Clean component composition using Shadcn UI and Lucide icons.
3. React-Redux state management for async API queries, polling intervals, and cache invalidation.
4. Solana Wallet Adapter (`@solana/wallet-adapter-react`) supporting Phantom, Solflare, etc.
5. All backend interactions flow through typed API client functions.
6. Zero private signing keys or backend secrets in browser bundle.

## Implementation Steps

1. Configure Next.js App Router and verify Shadcn UI components.
2. Build Redux store slices for markets, pools, telemetry, and agent state.
3. Implement `/markets` screen rendering real-time price feeds, premiums, and freshness indicators.
4. Implement `/launch` wizard with interactive regime parameterization and PDA preview.
5. Implement `/pool/[address]` dashboard with live charts, health meters, and agent badges.
6. Build `/demo` walkthrough page.
7. Run bundle inspection to verify zero secrets are packaged.

## Tests

- Responsive layout rendering tests across desktop and mobile viewports.
- Redux slice state transition and thunk handling tests.
- Form validation on pool launch inputs (non-zero supply, positive prices).
- Clean wallet connect and disconnect state handling.

## Verification Commands

```bash
pnpm --prefix apps/web lint
pnpm --prefix apps/web build
pnpm --prefix apps/web test
```

## Acceptance Criteria

- All 4 required screens (`/markets`, `/launch`, `/pool/[address]`, `/demo`) functional and navigable.
- Real-time metrics (reference price, tokenized price, premium, freshness, health, agent decision) displayed accurately.
- Absolutely **no provider secrets or private keys** present in browser bundle or source maps.

## Documentation Updates

- Update `ai/module-map.md` with new frontend routes and features.
- Update `ai/current-state.md` upon completion.

## Failure Conditions

- Inclusion of `PRO_API_KEY` or signing secrets in client bundle.
- Direct database access from frontend components.
- Broken responsive layouts or unhandled promise rejections on wallet disconnect.
