# Asset Structure & Token Representation

## 1. Executive Summary

This document specifies the asset representation architecture of **Equity Catalyst** for Shariah supervisory review under AAOIFI Shariah Standard No. 21 (*Financial Papers: Shares and Sukuk*) and International Islamic Fiqh Academy (IIFA) Resolution No. 63 (1/7) on Financial Markets.

Equity Catalyst is designed exclusively around **permissible spot equity ownership representation**. It operates as a non-leveraged market launch and liquidity platform on Solana, eliminating all forms of debt, leverage, margin, short-selling, interest-bearing mechanisms, and synthetic derivative contracts.

---

## 2. On-Chain Asset Representation Model

```
┌─────────────────────────────────────────────────────────────┐
│                    UNDERLYING REAL EQUITY                   │
│         (Audited, Regulated Custodian / SPV Holding)        │
└──────────────────────────────┬──────────────────────────────┘
                               │ 1:1 Direct Ownership Proof
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 TOKENIZED SPOT EQUITY (SPL)                 │
│      (e.g., AAPLx, MSFTx, NVDAx on Solana SPL Standard)      │
└──────────────────────────────┬──────────────────────────────┘
                               │ Continuous Screening & Pricing
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                   EQUITY CATALYST PLATFORM                  │
│       • Anchor Program (equity_vault)                        │
│       • Pyth Lazer Low-Latency Reference Feeds (200ms)      │
│       • Meteora Dynamic Bonding Curves (DBC - Spot Only)     │
│       • Risk Engine (100% Spot, Min Cash Reserve >= 10%)    │
└─────────────────────────────────────────────────────────────┘
```

### 2.1 Genuine Fractional Equity Ownership
- Each token (e.g., `AAPLx`, `MSFTx`) represents a direct fractional undivided ownership interest (*Musha'*) in the underlying shares held in insolvency-remote custody.
- The platform does not emit or trade synthetic debt instruments, contracts for difference (CFDs), futures, options, or debt-backed notes.
- In accordance with AAOIFI Standard No. 21 (Clause 2/1), a share represents a proportional undivided share in the company’s net assets, capital, revenues, and risks.

### 2.2 Custody & Legal Segregation
- Underlying shares are deposited with regulated custodians under bankruptcy-remote special purpose vehicles (SPVs) or licensed tokenized asset issuers (such as Backed Finance).
- Rehypothecation is strictly prohibited: shares cannot be pledged, loaned, lent to short-sellers, or encumbered to generate interest (*Riba*).

---

## 3. Financial Screening & Eligibility Criteria

Assets listed and supported on Equity Catalyst must satisfy three quantitative screening thresholds compliant with AAOIFI Standard No. 21:

| Screening Criterion | AAOIFI Standard 21 Rule | Equity Catalyst Enforcement |
| :--- | :--- | :--- |
| **Core Business Permissibility** | Impermissible core activities prohibited (conventional banking, alcohol, gambling, tobacco, weapons, adult entertainment). | On-chain asset registry whitelist with categorical exclusion. |
| **Interest-Bearing Debt Ratio** | Total interest-bearing debt / 12-month trailing market cap $< 30\%$. | Evaluated via financial data feeds prior to market pool creation. |
| **Interest-Bearing Cash Ratio** | Total cash and interest-bearing deposits / 12-month trailing market cap $< 30\%$. | Continuously checked; assets exceeding 30% are flagged for review. |
| **Impure Income Ratio** | Revenue derived from non-operating prohibited activities $< 5\%$ of total revenue. | Mandatory reporting for dividend purification requirements. |

---

## 4. Solana SPL Token Mint & Metadata Architecture

On Solana, each verified equity asset is represented by an immutable SPL Token Mint:
- **Token Standard**: Standard Solana Program Library (SPL) Token.
- **Decimals**: 6 decimals (matching $USDC$ quote asset for micro-precision).
- **Supply Constraints**: Minting authority is strictly tied to audited custodial deposits; unbacked token issuance is prevented at the protocol layer.
- **Freeze Authority**: Restricted exclusively to regulatory compliance and legal sanctions compliance, verifiable on-chain.
