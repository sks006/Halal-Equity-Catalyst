# ADR-006: Mainnet-First Canonical Tokenized Equity Assets

## Status
Accepted

## Context
A major failure mode in tokenized asset hackathons and prototypes is the invention of arbitrary "mock equity" tokens with custom decimals, zero liquidity, and no legal backing. This obscures real-world issues like token standards (SPL Token vs Token-2022), decimal discrepancies (8 vs 6 decimals), and real routing through DEX aggregators.

## Decision
We mandate a **Mainnet-First Canonical Asset Policy**:
- Only real, statutory tokenized equities issued by legitimate, audited issuers are supported.
- **Primary Canonical Asset**: **NVIDIA xStock (`NVDAx`)**
  - Issuer: **Backed Finance** (Backed Assets GmbH, Switzerland)
  - Legal Basis: Collateralized certificates under the Swiss DLT Act
  - Mint: `Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh`
  - Decimals: 8 (`TokenDecimal.EIGHT`)
  - Supported Quotes: `USDC` (6 decimals) and `WSOL` (9 decimals)
- Mock or randomly generated equity tokens are strictly prohibited in launch pipelines.

## Consequences
### Positive
- Accurate representation of institutional RWA settlement on Solana.
- Direct composability with existing mainnet liquidity on Raydium, Meteora, and Jupiter.

### Negative
- Testing requires handling real token decimal precision (8 decimals) and specific quote pairs.
