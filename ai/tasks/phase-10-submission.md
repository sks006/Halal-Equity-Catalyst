# Phase 10 — Submission

Status: IN_PROGRESS
Owner: unassigned
Dependencies: Phase 08, Phase 09
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [ ] verification
- [x] documentation
- [ ] review

## Objective

Package all hackathon deliverables, recorded demonstrations, architectural reviews, and public repository documentation for final submission.

## Context

Final packaging of the Equity Catalyst project, demonstrating institutional tokenized equity price discovery on Solana using Pyth Pro, Meteora Dynamic Bonding Curves, Backed Finance xStocks, an off-chain dual-line Risk Engine, and a Next.js App Router dashboard.

## Allowed Files

- `README.md`
- `walkthrough.md`
- `ai/**`
- `docs/**`
- `.tempmediaStorage/**`

## Forbidden Files

- Any remaining placeholder images or broken asset links
- Unverified transaction signatures in public submission materials
- Any exposed secrets in repository commit history

## Requirements

1. **Working Media Artifacts**:
   - High-fidelity end-to-end recorded walkthrough video (`frontend_walkthrough.webp`).
   - High-resolution UI screenshots:
     - Institutional landing page
     - Vault overview dashboard
     - Vault creation wizard with live PDA preview
     - Vault detail and position allocation table
     - Pyth portfolio telemetry
2. **Canonical Asset Verification**:
   - Full documentation of real on-chain mints (Backed NVIDIA `NVDAx`: `Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh`).
3. **Database Audit Proof**:
   - Documented PostgreSQL schema and confirmed rows from `dbc_pools` table.
4. **Clean Git Repository**:
   - Zero committed private keys, `.env` secrets, or untracked binary scratch files.

## Implementation Steps

1. Verify and clean root `README.md` with architectural diagrams, feature highlights, and quickstart commands.
2. Compile `walkthrough.md` embedding media artifacts, code snippets, and verified transaction signatures.
3. Conduct final pass over `ai/` context, ADRs, invariants, and runbooks.
4. Verify all tests pass across Rust workspace and Next.js frontend.
5. Record final submission commit.

## Tests

- Link checker verifying all relative and artifact markdown links in `README.md` and `walkthrough.md`.
- Media preview inspection ensuring embedded videos and images load without error.

## Verification Commands

```bash
cargo check --workspace
pnpm --prefix apps/web build
git status
```

## Acceptance Criteria

- Code compiles cleanly with zero errors across all packages.
- All media artifacts, walkthrough recordings, and screenshots are embedded and accessible.
- Repository documentation accurately represents actual working code.

## Documentation Updates

- Finalize `walkthrough.md` and `README.md`.
- Mark all completed milestones in `ai/current-state.md`.

## Failure Conditions

- Broken links or missing media artifacts in submission walkthrough.
- Committing unresolved or failing test suites.
- Fabricated or unqueryable mainnet/devnet evidence.
