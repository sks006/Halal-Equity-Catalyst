# Canonical Solana Mainnet & Oracle Verification Evidence

Last Verified: 2026-09-18
Verification Cluster: `mainnet-beta`
RPC Endpoint: `https://api.mainnet-beta.solana.com`
Oracle Service: Pyth Network Hermes (`https://hermes.pyth.network`)
Verification Tool: [`scripts/verify-mainnet.ts`](../../scripts/verify-mainnet.ts)

---

## 1. Verified Asset Identity (Backed Finance NVDAx)

| Field | On-Chain Verified Value | Verification Source | Status |
| :--- | :--- | :--- | :--- |
| **Asset Name** | Backed NVIDIA Corporation | Pyth Hermes & Backed Finance Swiss SPV | **VERIFIED** |
| **Symbol** | `NVDAx` | Canonical Token Registry | **VERIFIED** |
| **Mint Address** | `Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh` | Solana Mainnet RPC `getAccountInfo` | **VERIFIED** |
| **Decimals** | `6` | SPL Token-2022 Account Layout | **VERIFIED** |
| **Owner Program** | `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb` | Token-2022 Program | **VERIFIED** |
| **Account Data Length** | `679` bytes | Mainnet Account Header | **VERIFIED** |
| **Executable** | `false` | Account Metadata | **VERIFIED** |
| **Explorer Link** | [Solana Explorer: NVDAx Mint](https://explorer.solana.com/address/Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh) | Public Block Explorer | **RESOLVABLE** |

---

## 2. Verified Oracle Feed (Pyth Hermes)

| Field | Verified Value | Verification Source | Status |
| :--- | :--- | :--- | :--- |
| **Oracle Provider** | Pyth Network Hermes | Hermes REST API | **VERIFIED** |
| **Feed Identifier** | `b1073854ed24cbc755dc527418f52b7d271f6cc967bbf8d8129112b18860a593` | Pyth Production Feed Registry | **VERIFIED** |
| **Symbol** | `Equity.US.NVDA/USD` | Feed Attributes | **VERIFIED** |
| **Description** | `NVIDIA CORP / US DOLLAR` | Pyth Metadata | **VERIFIED** |
| **Trading Schedule** | `America/New_York;0930-1600,0930-1600,0930-1600,0930-1600,0930-1600,C,C` | Market Hours Specification | **VERIFIED** |
| **Explorer Link** | [Pyth Feed: NVDA/USD](https://pyth.network/price-feeds/equity-us-nvda-usd) | Pyth Network Explorer | **RESOLVABLE** |

---

## 3. Verified DEX Program Deployment (Meteora DBC)

| Field | On-Chain Verified Value | Verification Source | Status |
| :--- | :--- | :--- | :--- |
| **Program Name** | Meteora Dynamic Bonding Curve (DBC) | Solana Mainnet-Beta | **VERIFIED** |
| **Program ID** | `dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN` | Solana Mainnet RPC `getAccountInfo` | **VERIFIED** |
| **Executable** | `true` | Program Account Metadata | **VERIFIED** |
| **Explorer Link** | [Solana Explorer: Meteora DBC Program](https://explorer.solana.com/address/dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN) | Public Block Explorer | **RESOLVABLE** |

---

## 4. Operational Deployment Status & Blockers (Zero Fabricated Evidence)

In strict adherence to repository truth invariants, un-executed transactions and un-deployed pools are documented as pending without simulated or fabricated artifacts:

| Capability / Resource | Required Evidence | Current Status | Blocker Description |
| :--- | :--- | :--- | :--- |
| **Meteora Pool Account** | On-chain pool PDA | `PENDING_DEPLOYMENT` | Requires live capital wallet SOL funding to initialize a real mainnet DBC pool |
| **Base / Quote Vault Accounts** | On-chain token accounts | `PENDING_DEPLOYMENT` | Dependent on active pool creation |
| **Mainnet Transaction Signature** | 64-byte base58 signature | `NOT_SUBMITTED` | Real mainnet transaction has not been submitted; zero mock signatures allowed |
| **Transaction Finalization** | RPC slot confirmation | `NOT_SUBMITTED` | Dependent on real transaction broadcast |

---

## 5. Machine-Readable Verification Audit Run

Result of running `npx ts-node scripts/verify-mainnet.ts`:

```json
{
  "timestamp": "2026-09-18T03:37:38.923Z",
  "cluster": "mainnet-beta",
  "rpcEndpoint": "https://api.mainnet-beta.solana.com",
  "allPassed": false,
  "passedCount": 6,
  "totalCount": 10,
  "checks": [
    { "id": "ASSET_MINT_VALIDATION", "passed": true, "required": true },
    { "id": "MINT_ACCOUNT_INFO", "passed": true, "required": true },
    { "id": "TOKEN_PROGRAM_VALIDATION", "passed": true, "required": true },
    { "id": "PYTH_FEED_QUERY", "passed": true, "required": true },
    { "id": "PYTH_TIMESTAMP_FRESHNESS", "passed": true, "required": true },
    { "id": "METEORA_DBC_PROGRAM", "passed": true, "required": true },
    { "id": "METEORA_POOL_ACCOUNT", "passed": false, "required": true, "error": "No METEORA_POOL_ADDRESS configured. Pool deployment has not yet occurred on mainnet." },
    { "id": "VAULT_ACCOUNTS", "passed": false, "required": true, "error": "No pool address configured to query base and quote vault accounts" },
    { "id": "TRANSACTION_SIGNATURE", "passed": false, "required": true, "error": "No MAINNET_TX_SIGNATURE provided. Real transaction has not been submitted." },
    { "id": "TRANSACTION_FINALIZATION", "passed": false, "required": true, "error": "No transaction signature available to verify finalization." }
  ]
}
```
