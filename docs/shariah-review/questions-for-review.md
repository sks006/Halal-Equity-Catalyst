# Shariah Supervisory Board Review Questionnaire

## Notice of Purpose

This document outlines the specific supervisory questions, structural nuances, and technical parameters submitted to qualified Shariah scholars and supervisory boards reviewing the **Equity Catalyst** platform.

The objective is to establish whether the platform qualifies for formal **Shariah Compliance Certification** under AAOIFI Shariah Standards (notably Standard No. 21 and Standard No. 18) and relevant resolutions of the International Islamic Fiqh Academy (IIFA).

---

## Question 1: Constructive Possession (*Qabd Hukmi*) via High-Performance Blockchain State Finality

**Background**:
Under AAOIFI Shariah Standard No. 18, constructive possession requires that the purchaser obtain unimpeded control over the purchased asset, allowing them to freely utilize, transfer, or liquidate it without dependency on a third party.

On Solana, transactions execute atomically within a single block (~400ms finality). When a user executes a spot swap ($USDC \leftrightarrow \text{Tokenized Equity}$), the token balance is credited directly to the user's non-custodial Associated Token Account (ATA). The user has exclusive cryptographic private key control over these tokens.

**Question for Scholars**:
> *Does cryptographic state confirmation and direct non-custodial wallet custody constitute valid constructive possession (Qabd Hukmi) under Shariah law, fulfilling the requirements of simultaneous exchange (Taqaabud) in spot transactions?*

---

## Question 2: Algorithmic Bonding Curve Pricing (Meteora Dynamic Bonding Curves) vs. Market Reference Fair Value

**Background**:
Meteora Dynamic Bonding Curves (DBC) utilize deterministic mathematical curves to calculate spot prices based on reserve pool ratios. To prevent excessive pricing distortion (*Ghabn Fahish*), Equity Catalyst anchors DBC pool pricing to **Pyth Lazer** real-time reference market data streams (200ms fixed update frequency).

If the spread between the bonding curve price and the reference market price exceeds pre-defined safety bounds, trade execution is paused or rejected.

**Question for Scholars**:
> *Is the combination of deterministic bonding curve pricing bounded by external real-time reference feeds acceptable as a fair-value price discovery mechanism, provided both parties are informed of the effective price and price impact prior to trade confirmation?*

---

## Question 3: Automated Dividend Purification Reporting

**Background**:
In compliance with AAOIFI Standard No. 21, companies whose core business is permissible may still derive a small percentage ($< 5\%$) of non-operating interest income or impermissible revenue. Investors are required to purify this portion of their dividend proceeds by donating it to charitable causes without seeking spiritual reward (*Thawab*).

Equity Catalyst calculates and displays this purification ratio on-chain and in user interfaces, alerting investors of the exact percentage required for purification upon dividend distribution.

**Question for Scholars**:
> *Does the provision of automated calculation and transparency of impure dividend percentages fulfill the platform's fiduciary obligations under Shariah, or is the platform required to execute automated charitable deduction at the protocol layer before distribution?*

---

## Question 4: Characterization of Platform Facilitation Fees (*Ujrah*)

**Background**:
Equity Catalyst extracts a total fee of $0.15\%$ ($15\text{ bps}$) per spot transaction, transparently unbundled into:
1. Pool Liquidity Fee ($0.10\%$): Paid to liquidity providers for capital inventory provision.
2. Platform Controller Fee ($0.03\%$): Paid for on-chain risk defense and policy enforcement.
3. Execution & Oracle Fee ($0.02\%$): Paid for Pyth Lazer oracle cryptographic verification and network compute.

No borrow fees, rollover interest, or liquidation penalties exist.

**Question for Scholars**:
> *Are these three fee components recognized as valid service consideration (Ujrah) in exchange for specific, tangible administrative, computational, and liquidity facilitation services?*

---

## Question 5: Tokenized Equity Legal Segregation & Insolvency Remoteness

**Background**:
The platform utilizes tokenized equities backed by regulated custodial entities (e.g., Backed Finance / licensed custodians) where underlying physical shares are held in bankruptcy-remote SPVs.

**Question for Scholars**:
> *Does the legal segregation of underlying shares in bankruptcy-remote vehicles adequately protect the property rights (Haqq al-Milkiyyah) of token holders under Shariah principles, ensuring that token holders are treated as genuine beneficial owners rather than general unsecured creditors in the event of an issuer insolvency?*
