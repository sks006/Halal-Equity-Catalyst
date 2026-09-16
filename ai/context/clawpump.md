# ClawPump & Bonding Curve Mechanics

## Overview
Bonding curve launchpads (Pump.fun, ClawPump) pioneered fair-launch mechanics where tokens are issued on a virtual bonding curve until a target market capitalization is achieved, at which point liquidity is migrated to an AMM (Raydium or Meteora).

## Evolution in Equity Catalyst
While meme launchpads use simple hyperbolic or exponential curves with extreme early slippage and sniper vulnerability, Equity Catalyst upgrades this paradigm for equities:
1. **Fair Value Anchoring**: Rooted in Pyth Pro oracle data rather than arbitrary zero-price origin.
2. **Piecewise Concentration**: Liquidity depth is concentrated around the anchor price (Regime B) rather than uniformly thinned.
3. **Anti-Snipe Linear Scheduler**: Starting fees decay linearly to eliminate MEV bot extraction at block 0.
