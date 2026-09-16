# Domain

## Asset

A tokenized/pre-IPO financial asset.

## Reference Price

Price representing the underlying/reference market.

Primary source:
Pyth Pro.

## Tokenized Price

On-chain/RWA representation price.

Examples:

- xStock
- Ondo
- PreStocks
- Tessera

## Premium

Difference between tokenized price and reference price.

```
premium_bps =
(token_price - reference_price)
* 10_000
/ reference_price
```

## Equity Health

Current market-health state.

States:

- HEALTHY
- WARNING
- CRITICAL

## DBC

Meteora Dynamic Bonding Curve.

## Graduation

Transition from DBC launch liquidity to the configured migration destination.

## Agent

A bounded decision system.

Agent can:

- observe
- evaluate
- recommend
- request execution

Agent cannot bypass:

- risk engine
- policy
- authorization
- on-chain invariants
