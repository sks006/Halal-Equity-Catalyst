# Phase 00 — Foundation

Status: DONE
Owner: Agent 0
Dependencies: None
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [x] verification
- [x] documentation
- [x] review

## Objective

Make the repository compile and establish the Rust workspace.

## Context

Establishing the multi-language repository foundation across:
- Root Cargo workspace
- `apps/api` (Axum/Tokio backend service)
- `crates/shared` (pure deterministic domain logic)
- `programs/equity_vault` (Anchor on-chain smart contract)
- `apps/web` (Next.js frontend)
- Environment configuration and CI basics

## Allowed Files

- `Cargo.toml`
- `Cargo.lock`
- `crates/shared/**`
- `apps/api/**`
- `programs/equity_vault/**`
- `apps/web/**`
- `.env.example`
- `.gitignore`
- `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`

## Forbidden Files

- Production `.env` containing real secrets
- Private key files (*.json, *.pem, *.key)
- Production signing keypairs
- Direct edits to vendor or generated directories

## Requirements

1. Root Cargo workspace connecting `crates/shared`, `apps/api`, `programs/equity_vault`, and `integrations/*`.
2. `apps/api` crate configured with Axum, Tokio, and modular routing.
3. `crates/shared` crate containing pure deterministic domain logic without Axum, Tokio tasks, HTTP clients, SQL, or secrets.
4. `programs/equity_vault` Anchor program compiling with Anchor 0.30+.
5. `apps/web` Next.js App Router project initialized with TypeScript and strict mode.
6. Environment configuration template (`.env.example`) documenting all required variables.
7. CI verification basics.

## Implementation Steps

1. Configure root `Cargo.toml` workspace members and shared dependency versions.
2. Implement `crates/shared` domain math, types, and error definitions.
3. Scaffold `apps/api` with configuration parsing, error handling, state container, and router.
4. Verify `programs/equity_vault` Anchor program structure, accounts, and instructions.
5. Initialize and verify `apps/web` dependencies, TypeScript config, and Tailwind styling.
6. Create comprehensive `.env.example` with sanitized defaults.
7. Run workspace checks and test suites.

## Tests

- `crates/shared` domain and fixed-point math unit tests.
- `apps/api` router bootstrapping tests.
- Anchor program unit tests.
- Frontend build validation.

## Verification Commands

```bash
cargo check --workspace
cargo test --workspace

pnpm install
pnpm lint
pnpm build
```

## Acceptance Criteria

- Rust workspace builds cleanly without errors (`cargo check --workspace`).
- Workspace tests pass (`cargo test --workspace`).
- Frontend builds cleanly (`pnpm build`).
- No secrets or private signing keys committed.

## Documentation Updates

- Update `ai/current-state.md` upon completion.
- Document any platform-specific prerequisites in `ai/debug/runbook.md`.

## Failure Conditions

- Workspace compilation failure or circular dependencies.
- Leaking async runtimes (Tokio), HTTP clients (reqwest), or database drivers into `crates/shared`.
- Presence of committed API keys, private keys, or wallet seed phrases.
- Failing frontend builds or unhandled lint errors.
