# Spot Transaction & Execution Flow

## 1. Operating Lifecycle ($T+0$ Spot Settlement)

In compliance with Shariah principles governing spot transactions (*Bay' al-Murabaha / Bay' al-Hadir*), all exchange on Equity Catalyst operates on a **strictly synchronous, simultaneous settlement model ($T+0$)**.

```
┌─────────────────┐       USDC (Quote Asset)       ┌───────────────────────┐
│     Trader      │ ─────────────────────────────> │  Meteora DBC Spot     │
│   (Investor)    │ <───────────────────────────── │  Liquidity Pool       │
└─────────────────┘      Tokenized Spot Equity     └───────────────────────┘
                                   │
                                   │ Real-Time Verification
                                   ▼
                         ┌───────────────────┐
                         │    Pyth Lazer     │
                         │ Reference Pricing │
                         │ (200ms Heartbeat) │
                         └───────────────────┘
```

---

## 2. Step-by-Step Spot Settlement Process

### Step 1: Pre-Trade Policy & Risk Verification
Before any transaction instruction is executed:
- The risk engine verifies that the target asset is on the Shariah-eligible whitelist.
- The vault's unencumbered cash reserve is checked:
  $$\text{Cash Reserve Ratio} = \frac{\text{Unencumbered Liquid Cash}}{\text{Total Vault Portfolio Value}} \ge \text{min\_cash\_bps}$$
- **Zero Leverage Constraint**: Checks confirm no borrowing or margin is utilized. If borrowing is detected, the transaction errors with `ProhibitedLeverageOrShort`.

### Step 2: Pyth Lazer Low-Latency Reference Price Check
- Real-time reference pricing is queried from Pyth Lazer WebSocket streaming (channel `fixed_rate@200ms`).
- Execution bounds verify that the Meteora DBC curve price does not exceed tolerable oracle spread bounds (preventing *Ghabn Fahish* / excessive unilateral pricing distortion).

### Step 3: Simultaneous Delivery and Payment (*Taqaabud*)
- On Solana, token transfers execute atomically in a single block transaction via CPI:
  1. Buyer transfers $USDC$ to the liquidity pool / vault.
  2. The pool simultaneously delivers tokenized equity to the buyer’s Associated Token Account (ATA).
- Settlement is finalized deterministically upon Solana slot confirmation (~400ms), fulfilling constructive possession (*Qabd Hukmi*).

### Step 4: Post-Trade Immutability & Audit Trail
- An on-chain execution receipt event is emitted.
- All fee deductions (Pool LP fee, Platform fee, Execution fee) are accounted for transparently and immutably in the Solana ledger.

---

## 3. Absence of Postponed Reciprocal Exchange (*Sarf* Compliance)
- When swapping tokenized equities against $USDC$ (a currency/numeraire representation), exchange is executed on the spot without deferral of either counter-value (*Nasi'ah*).
- Both counter-values are transferred within the single atomic transaction bundle.
