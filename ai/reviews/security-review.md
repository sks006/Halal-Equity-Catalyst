# Security & Risk Review

## Threat Model Assessment

### 1. LLM Prompt Injection & Unauthorized Transactions
- **Risk**: Malicious prompt input causes an autonomous agent to propose transferring user funds to an attacker's address.
- **Mitigation**: Dual-line defense. Off-chain `RiskEngine` rejects any trade outside whitelist policy parameters. The on-chain Anchor program verifies that caller authorities, recipient vaults, and token mints are strictly registered.

### 2. Flash Loan Price Manipulation & Stale Oracles
- **Risk**: Attacker manipulates spot DEX price to trigger wrongful liquidation or extract undercollateralized loans.
- **Mitigation**: Pyth Pro confidence bounds ($\frac{\text{conf}}{\text{price}} \le 2\%$) and 60-second freshness checks prevent execution during abnormal spreads.

### 3. Reentrancy & Cross-Program Invocations (CPI)
- **Risk**: Reentrancy on vault share withdrawal or deposit.
- **Mitigation**: Anchor state updates follow the Checks-Effects-Interactions pattern. Vault balances and shares are mutated before token transfers execute.

### 4. Key Management & Exposure
- **Risk**: Leakage of transaction signing keys in API logs or client responses.
- **Mitigation**: Signer logic is isolated to `apps/api/src/engines/decision_engine/signer.rs`. No key material is serialized or exposed via Axum JSON endpoints.
