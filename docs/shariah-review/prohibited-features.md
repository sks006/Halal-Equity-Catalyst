# Permanently Eliminated Prohibited Features

## 1. Executive Summary & Verification Matrix

To ensure unconditional alignment with AAOIFI standards and classical Islamic commercial law, the Equity Catalyst protocol was fundamentally re-architected to purge all forms of leverage, margin loans, interest accrual, short-selling, and synthetic exposure.

The following matrix documents the architectural changes and their on-chain verification:

| Prohibited Feature | Classical Shariah Ground | Legacy System Status | Current System Status | Enforcing Layer |
| :--- | :--- | :--- | :--- | :--- |
| **Borrowing & Lending** | *Riba al-Qard* (Interest on Loans) | Collateralized borrow pools (`loan.rs`, `CreditClient`) | **Permanently Removed.** All loan state, instructions, and schemas deleted. | Anchor Program, Shared Rust Crates, TypeScript SDK |
| **Margin Leverage** | Speculative excess (*Gharar* & Debt Amplification) | Dynamic LTV borrowing up to 85% | **Permanently Banned.** Enforced $100\%$ equity capitalization ($0\%$ leverage). | `RiskEngine`, `validation.rs` (`ProhibitedLeverageOrShort`) |
| **Short Selling** | *Bay' ma la Yamlik* (Selling what one does not own) | Synthetic short execution | **Permanently Prohibited.** Trades strictly require prior unencumbered asset balance. | Spot-only validation checks |
| **Derivatives / Synthetics** | Gambling (*Maysir*) & Detached Risk | Cash-settled synthetic balance trackers | **Replaced by Genuine Spot.** Underlying asset-backed tokenized equities. | SPV custodial backing & SPL tokens |
| **Compounding Fees / Liquidations** | Unjust enrichment (*Akl Amwal bi al-Batil*) | Liquidation penalty cascades | **Eliminated.** Zero liquidation mechanics exist in spot equities. | Protocol architecture |

---

## 2. On-Chain Cryptographic Proof of Absence

### 2.1 Anchor Program (`equity_vault`)
1. **Deletion of Loan State**: The Anchor account `Loan` and PDA seed derivation `["loan", vault, borrower]` have been purged.
2. **Replacement of Max LTV with Minimum Cash Reserve**:
   - The on-chain `Policy` account no longer tracks `max_ltv_bps`.
   - It now enforces `min_cash_bps: u16`, requiring vaults to maintain unencumbered liquid liquidity to prevent over-extension.
3. **Execution Errors**:
   - `6004 BelowMinCashReserve`: Triggered if cash drops below mandatory reserve.
   - `6005 ProhibitedLeverageOrShort`: Explicitly blocks any instruction attempting leverage, short selling, or debt creation.

### 2.2 Shared Core Engine (`crates/shared`)
- In `risk.rs`: Functions `calculate_ltv` and `check_ltv_limit` were replaced with `calculate_cash_reserve`, `check_cash_reserve`, and `enforce_spot_only`.
- In `validation.rs`: `ValidationError::ExceedsMaxLtv` was replaced with `ValidationError::BelowMinCashReserve`, `ValidationError::ProhibitedLeverageOrShort`, and `ValidationError::IneligibleShariahAsset`.

### 2.3 API & Worker Pipeline (`apps/api`)
- The risk engine `ltv.rs` worker was permanently deleted.
- Database migration `008_remove_ltv_spot_policy.sql` purged the `loans` table and altered `policies` to replace `max_ltv_bps` with `min_cash_bps`.
- All execution quotes enforce zero-debt verification prior to signing.
