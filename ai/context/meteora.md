# Meteora Dynamic Bonding Curve (DBC) Reference

## Program Architecture
- **Program ID**: `dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN` (Devnet & Mainnet)
- **SDK**: `@meteora-ag/dynamic-bonding-curve-sdk`

## Key SDK Methods & PDAs
- **Pool PDA Derivation**:
  ```typescript
  deriveDbcPoolAddress(quoteMint: PublicKey, baseMint: PublicKey, config: PublicKey): PublicKey
  ```
- **Token Vault PDA Derivation**:
  ```typescript
  deriveDbcTokenVaultAddress(pool: PublicKey, mint: PublicKey): PublicKey
  ```
- **Composite Creation**:
  ```typescript
  client.partner.createConfigAndPool({ ...params }): Promise<Transaction>
  ```

## 3-Regime Piecewise Curve Parameters
- **`buildCurveWithCustomSqrtPrices`**: Accepts an array of Q64.64 sqrt prices and corresponding liquidity weights.
- **Dynamic Fee Mode**: Enables volatility-based fee adjustments using exponential moving average (EMA) transaction volume.
- **Migration**: Automated graduation into Meteora DAMM v2 upon hitting target quote reserves.
