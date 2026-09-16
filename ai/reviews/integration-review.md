# Integration Review

## External Protocols & Venues

### 1. Pyth Pro / Hermes (`integrations/pyth`)
- **Status**: Verified.
- **Protocol**: HTTPS REST (`/v2/updates/price/latest`).
- **Capabilities**: Normalizes exponents, parses confidence intervals, and maps equity feed IDs.

### 2. Meteora DBC (`sdk/src/meteora`)
- **Status**: Verified.
- **SDK**: `@meteora-ag/dynamic-bonding-curve-sdk` v1.5.12.
- **Capabilities**: 3-regime piecewise curve generation, dynamic anti-snipe fee decay, DAMM v2 migration parameters, deterministic PDA derivation for pools and token vaults.

### 3. Jupiter v6 DEX Aggregator (`integrations/jupiter`)
- **Status**: Verified.
- **Protocol**: HTTPS REST (`https://quote-api.jup.ag/v6`).
- **Capabilities**: Slippage analysis, route discovery, and quote-only risk evaluation.
