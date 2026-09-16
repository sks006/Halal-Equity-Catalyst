# ADR-003: Meteora DBC with 3-Regime Equity Discovery Curve

## Status
Accepted

## Context
Standard constant-product bonding curves ($x \cdot y = k$) or meme-coin launch curves generate severe slippage for large institutional orders and lack the structure needed for tokenized equity assets with known fair-value anchors.

## Decision
We leverage the **Meteora Dynamic Bonding Curve (DBC)** SDK (`@meteora-ag/dynamic-bonding-curve-sdk`) with a custom **3-Regime Piecewise Discovery Curve**:
1. **Regime A (Bootstrapping / Floor to Parity, $0.85 \times P_{anchor}$ to $1.00 \times P_{anchor}$)**:
   - Base weight 1x.
   - Dynamic anti-snipe linear fee scheduler decaying from 250 bps to 50 bps over 24 hours.
2. **Regime B (Active Discovery / Parity to $1.30 \times P_{anchor}$)**:
   - 4x concentrated liquidity depth centered on the anchor fair value.
   - Low price impact for retail and institutional blocks.
3. **Regime C (Mature Buffer / $1.30 \times P_{anchor}$ to $1.50 \times P_{anchor}$)**:
   - 8x concentrated liquidity depth acting as a volatility buffer prior to DAMM v2 migration.
4. **Graduation**:
   - 100% of migrated LP locked or routed directly to the equity vault.
   - Dynamic volatility fee engine enabled in both DBC and DAMM v2 pools.

## Consequences
### Positive
- Deep liquidity where trading naturally clusters around fair value.
- Protection against front-running and MEV snipers at pool inception.
- Frictionless, automated migration path into Meteora DAMM v2.

### Negative
- Requires careful parameterization of price checkpoints and supply distribution.
