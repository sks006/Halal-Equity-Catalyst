# Equity Discovery Curve (EDC) — Design & Research Specification

> **Core Philosophy**: *"The novelty is not that we simply changed the curve; it is that we designed the curve around the behavior of tokenized-stock markets rather than meme-token speculation."*

---

## 1. Executive Summary & Problem Context

Traditional bonding curves on Solana (e.g., pump.fun, standard single-segment constant product or exponential curves) were built for **meme-token speculation**. Their characteristics are deliberately extreme:
- **Zero Fundamental Anchor**: Tokens start near $0.000000001 with unknown limits.
- **Hyperbolic Price Sensitivity**: Infinitesimal buy orders double or quadruple the price within seconds to trigger speculative FOMO.
- **Thin, Uniform/Degrading Liquidity**: Liquidity depth is paper-thin throughout the curve, inviting sniper bots to frontrun retail and dump within blocks.
- **Abrupt AMM Cliff**: The migration threshold triggers an instantaneous transition into a standard AMM where pool depth and price volatility undergo a violent shock.

### Why Tokenized Stock Markets Require a Different Architecture

Tokenized stocks (synthetic equities, pre-IPO shares, real-world asset equity representations) have fundamentally different economic realities:
1. **Fundamental Valuation Anchor**: Unlike meme coins, tokenized stocks track or converge toward real enterprise value, reference oracle feeds (e.g., Pyth NVDA, AAPL feeds), or net asset value (NAV).
2. **Institutional & Treasury Order Sizing**: Real participants transact in non-trivial block sizes ($1,000 to $100,000+). A 100x slippage on a $5,000 order makes the market unviable for real capital.
3. **Continuous Discovery vs. Pump-and-Dump**: The goal of an equity launch is orderly capital formation, continuous price discovery, and minimal adverse selection against liquidity providers.
4. **Corporate Action Volatility**: Equities experience scheduled discrete information shocks (e.g., quarterly earnings releases, guidance updates, regulatory approvals). Dynamic fee protection is required to protect liquidity from latency arbitrageurs.

---

## 2. The Three Regimes of the Equity Discovery Curve

Meteora's Dynamic Bonding Curve (DBC) supports **arbitrary piecewise curve segments** defined by checkpoint square-root prices and custom **liquidity weights** ($W_i$). We leverage this capability to engineer three distinct operational regimes:

```
Price (Quote / Base)
  ▲
  │                                                  [Regime C: Mature Market]
  │                                                  High Depth / Buffer
  │                                              ┌───────────────────────► Migration (DAMM v2)
  │                                              │ (Slope: Very Low)
  │                       [Regime B: Active]     │
  │                       Deep Discovery Band    │
  │                   ┌──────────────────────────┘
  │                   │ (Slope: Low / Controlled)
  │   [Regime A]      │
  │   Bootstrapping   │
  │  ─────────────────┘ (Slope: Moderate)
  │  P_floor
──┴────────────────────────────────────────────────────────────────────────► Cumulative Volume / Quote
     Regime A: Launch       Regime B: Discovery         Regime C: Mature
```

---

### Regime A: Launch & Controlled Bootstrapping
* **Price Range**: $[P_0, P_1]$ (e.g., $0.80 \cdot P_{\text{anchor}} \to 1.00 \cdot P_{\text{anchor}}$)
* **Liquidity Weight**: $W_A = 1$ (Baseline Weight)
* **Economic Objective**:
  - Provide an initial bootstrapping window where early participants can acquire shares at a modest discount to reference anchor without triggering wild vertical price spikes.
  - Mitigate sniper bot extraction: The price moves in a measured, predictable fashion rather than 100x in one block.
  - Paired with an **Exponential / Linear Fee Scheduler** that starts higher (e.g., 200–300 bps) during the opening window to make toxic atomic frontrunning unprofitable.

---

### Regime B: Active Price Discovery
* **Price Range**: $[P_1, P_2]$ (e.g., $1.00 \cdot P_{\text{anchor}} \to 1.30 \cdot P_{\text{anchor}}$)
* **Liquidity Weight**: $W_B \gg W_A$ (e.g., $W_B = 3 \times \text{ to } 5 \times W_A$)
* **Economic Objective**:
  - Concentrates the vast majority of bonding curve depth around the asset's **consensus fair-value band**.
  - Traditional financial market order books exhibit maximum depth around the current mid-price. By allocating significantly higher liquidity weight to this regime, large orders ($5k–$50k) can execute with low, predictable slippage ($\le 1.5\%$).
  - Enables two-way trading (both buying and selling) where prices fluctuate organically in response to market news rather than liquidity exhaustion.

---

### Regime C: Mature Market (Pre-Graduation Buffer)
* **Price Range**: $[P_2, P_3]$ (e.g., $1.30 \cdot P_{\text{anchor}} \to 1.50 \cdot P_{\text{anchor}}$)
* **Liquidity Weight**: $W_C > W_B$ (e.g., $W_C = 8 \times \text{ to } 10 \times W_A$)
* **Economic Objective**:
  - Pre-graduation stabilization zone as the pool approaches the **Meteora DAMM v2 migration quote threshold**.
  - Eliminates the notorious "graduation cliff" where traders pump a curve over the migration line to dump immediately on the AMM.
  - Deep liquidity dampens terminal volatility, ensuring the asset transitions into Meteora DAMM v2 with pricing and depth that perfectly match the post-migration AMM pool configuration.

---

## 3. Mathematical Formalization

### 3.1 Piecewise Concentrated Liquidity Formulation

Let:
- $P_0 < P_1 < P_2 < P_3$ be the price checkpoints across the 3 regimes.
- $\sqrt{P_i}$ be the square-root prices (in Q64.64 representation).
- $W = [W_A, W_B, W_C]$ be the relative liquidity weights for Regimes A, B, and C.
- $L_0$ be the base liquidity scalar solved by the DBC builder.
- $L_i = W_i \cdot L_0$ be the effective virtual liquidity in segment $i \in \{A, B, C\}$.

For any price transition from $P_{i-1}$ to $P_i$ within segment $i$:
1. **Base Token Consumption** (tokens dispensed by curve):
   $$\Delta X_i = L_i \left( \frac{1}{\sqrt{P_{i-1}}} - \frac{1}{\sqrt{P_i}} \right) = L_i \frac{\sqrt{P_i} - \sqrt{P_{i-1}}}{\sqrt{P_{i-1} P_i}}$$

2. **Quote Token Inflow** (quote currency deposited by traders):
   $$\Delta Y_i = L_i \left( \sqrt{P_i} - \sqrt{P_{i-1}} \right)$$

3. **Marginal Price Impact (Slippage)**:
   Given $P(Y) = \left( \sqrt{P_{i-1}} + \frac{Y}{L_i} \right)^2$, the marginal price movement per unit quote inflow is:
   $$\frac{dP}{dY} = \frac{2 \sqrt{P}}{L_i} = \frac{2 \sqrt{P}}{W_i L_0}$$

   > **Crucial Relationship**: The price impact $\frac{dP}{dY}$ is inversely proportional to $W_i$. By setting $W_C > W_B > W_A$, we guarantee that as the market matures and volume grows, each incremental dollar incurs exponentially lower price disruption.

---

## 4. Supporting Tooling & Parameter Configuration

| Parameter | Meme-Token Bonding Curve | Equity Discovery Curve (EDC) | Rationale |
| :--- | :--- | :--- | :--- |
| **Quote Token** | Wrapped SOL (WSOL) | **USDC** (Devnet/Mainnet SPL) | Avoids confounding stock price discovery with crypto-beta and SOL price swings. |
| **Regime Segments** | 1 (Monolithic / Steep) | **3 Regimes** (Launch, Discovery, Mature) | Reflects real order book depth distribution around fundamental valuation. |
| **Liquidity Distribution** | Decreasing / Paper-thin | **Increasing Depth** ($W_A: 1, W_B: 4, W_C: 8$) | Stabilizes trading and reduces slippage as position sizes scale. |
| **Base Fee Mode** | Static 100 bps | **Fee Scheduler Linear/Exponential** | 250 bps start decaying to 25–50 bps to disincentivize sniper MEV. |
| **Dynamic Fee** | Disabled | **Enabled** (Volatility Accumulator) | Widens fee band during earnings announcements / macro news shocks. |
| **Migration Target** | Raydium v4 / Generic AMM | **Meteora DAMM v2** (Customizable Fee) | Seamless transition into concentrated liquidity with customizable fee tiers. |
| **Token Standard** | Standard SPL Token | **SPL Token / Token2022** | Compatible with transfer hooks and corporate action auditing. |

---

## 5. Summary of Novelty

1. **Valuation-Centric Construction**: Parametrized from an oracle anchor price ($P_{\text{anchor}}$) rather than arbitrary zero-bound curves.
2. **Order-Book Emulation**: Replicates concentrated liquidity order-book depth ($L_B, L_C$) on a passive bonding curve mechanism.
3. **Volatility Shielding**: Uses Meteora DBC's dynamic volatility fee engine to adapt pool fee spreads dynamically when equity volatility spikes.
