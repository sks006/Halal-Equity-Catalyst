# Transparent Fee Structure & Zero-Interest Economics

## 1. Overview & Shariah Rationale

Islamic commercial jurisprudence requires that all fees represent consideration for genuine economic services (*Ujrah*), administrative facilitation, or asset liquidity provision. Fees must be clearly defined, predictable, and free from ambiguity (*Gharar*), interest (*Riba*), and punitive compounding penalties.

Equity Catalyst enforces a 100% transparent fee structure with zero hidden financing spreads, zero carry charges, and zero borrow interest.

---

## 2. Fee Breakdown Table

| Fee Component | Nominal Rate | Basis Points (bps) | Economic Purpose & Justification | Recipient |
| :--- | :--- | :--- | :--- | :--- |
| **Pool Liquidity Fee** | $0.10\%$ | $10\text{ bps}$ | Consideration for pool liquidity provision (*Ujrah li-Tawfir al-Siyulah*). Compensates LPs for inventory exposure. | Meteora DBC Pool Liquidity Providers |
| **Platform Controller Fee** | $0.03\%$ | $3\text{ bps}$ | Service fee for on-chain risk evaluation, Shariah whitelist verification, and policy guardrails. | Protocol Treasury |
| **Execution & Oracle Fee** | $0.02\%$ | $2\text{ bps}$ | Reimbursement for Pyth Lazer continuous data stream validation, cryptographic checks, and Solana gas. | Node / Execution Validator |
| **Total Transaction Fee** | **$0.15\%$** | **$15\text{ bps}$** | **Total all-inclusive fee for spot execution.** | - |

---

## 3. Explicit Prohibitions & Absences

The following conventional fee structures are strictly excluded from Equity Catalyst:

1. **No Overnight Financing / Swap Rates**: Conventional brokers charge daily rollover interest (*Riba al-Qard*) on margin positions. Because Equity Catalyst does not offer leverage or borrowing, zero financing charges exist.
2. **No Borrowing Spreads / Collateral Fees**: No interest is accrued or charged against vault balances.
3. **No Liquidation Penalties**: In leveraged protocols, forced liquidations extract 5% to 15% punitive fees. In spot equity trading, assets cannot be liquidated against debt because no debt exists.
4. **No Hidden Markup (*Ghabn*)**: All exchange prices are anchored to live Pyth Lazer reference feeds with transparent price impact calculations displayed prior to execution.
