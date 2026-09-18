# Phase 08 — Mainnet

Status: IN_PROGRESS
Owner: unassigned
Dependencies: Phase 03, Phase 06
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [x] verification (Phase 08A Real Verification Gate implemented and validated)
- [x] documentation
- [ ] review

## Objective

Produce independently verifiable mainnet evidence. Zero simulated or fabricated signatures.

## Context

All claims of functionality, asset tokenization, bonding curve liquidity, and oracle pricing must be backed by real, queryable on-chain evidence on the Solana blockchain. Synthetic tokens, mock addresses, or unverifiable signatures are strictly unacceptable.

## Verification Checklist

- [x] **Asset Verified**: Real statutory tokenized stock verified on Solana RPC (`NVDAx`, mint: `Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh`, 8 decimals, Token-2022 program `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`).
- [x] **Pyth Feed Verified**: Active Pyth price feed ID queried from Hermes (`b1073854ed24cbc755dc527418f52b7d271f6cc967bbf8d8129112b18860a593`, `Equity.US.NVDA/USD` verified with market hours schedule).
- [x] **Meteora DBC Program Verified**: Program account verified executable on Solana RPC (`dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN`).
- [x] **Compliance Authorization Gate Verified**: `set_asset_compliance` locked to vault authority (`has_one = authority @ VaultError::UnauthorizedKeeper`), validated in Anchor integration tests.
- [x] **Zero Fabricated Evidence Enforced**: `scripts/launch-real-pool.ts` and `scripts/verify-mainnet.ts` strictly enforce `NO RPC-CONFIRMED TRANSACTION => NO TRANSACTION SIGNATURE STORED => NO "REAL POOL LAUNCHED" MESSAGE`.
- [ ] **DBC Config Deployed**: On-chain config account created under Meteora DBC Program (Blocked: requires funded mainnet signer).
- [ ] **Pool Deployed**: Deterministically derived pool PDA deployed and queryable via RPC (Blocked: requires funded mainnet signer).
- [ ] **Stock / Quote Pair Initialized**: Base token vault and quote token vault accounts initialized on mainnet.
- [ ] **Swap Confirmed**: Real swap transaction executed and confirmed on-chain.
- [ ] **Pool State Verified**: Post-swap reserves, Q64.64 sqrtPrice, and curve progress read from blockchain.
- [ ] **Dashboard Shows Live Data**: Web frontend connects to live RPC and displays verified state.
- [ ] **Transaction Signatures Documented**: Valid transaction hashes recorded in PostgreSQL and documentation.

## Allowed Files

- `scripts/verify-mainnet.ts`
- `scripts/launch-real-pool.ts`
- `ai/context/**`
- `ai/current-state.md`
- `docs/**`

## Forbidden Files

- Creating or committing mock/fake token mint addresses disguised as real equities
- Fabricating transaction signatures in documentation or tests

## Requirements

1. **No Fake Evidence**: Every address, account, PDA, and transaction signature must be independently queryable on Solana Explorer (`explorer.solana.com`) or Solscan.
2. **Canonical Asset Only**: Use verified statutory assets (Backed Finance xStocks).
3. **Audit Trail**: Every transaction signature must be persisted in PostgreSQL and cross-referenced in repository documentation.

## Implementation Steps

1. Verify target tokenized stock asset parameters against Solana mainnet state.
2. Query live Pyth Hermes oracle endpoint for current price and confidence.
3. Build and execute real DBC pool launch transaction on live network.
4. Execute test swap transaction verifying AMM curve math on-chain.
5. Fetch and document full account dump and transaction signatures.
6. Verify live rendering in the web dashboard.

## Tests

- Query on-chain pool account via `solana-web3.js` `Connection.getAccountInfo`.
- Query transaction status via `Connection.getSignatureStatus`.
- Assert base token vault balance matches expected reserve.

## Verification Commands

```bash
solana account <POOL_ADDRESS> --url mainnet-beta
solana confirm -v <TRANSACTION_SIGNATURE> --url mainnet-beta
npx ts-node scripts/launch-real-pool.ts
```

## Acceptance Criteria

- All items in the verification checklist confirmed.
- Every transaction signature, pool address, and config address is queryable on Solana Explorer.
- Zero mock or fabricated evidence.

## Documentation Updates

- Document confirmed transaction signatures and pool addresses in `ai/current-state.md`.
- Update `walkthrough.md` with explorer links and screenshots.

## Failure Conditions

- Any claim of mainnet or live verification that cannot be resolved on a public Solana RPC.
- Inventing mock tokens with custom symbols or fake collateral.
