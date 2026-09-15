import { PublicKey } from "@solana/web3.js";

/**
 * Meteora Dynamic Bonding Curve (DBC) Program ID for Mainnet and Devnet.
 */
export const METEORA_DBC_PROGRAM_ID = new PublicKey(
  "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"
);

/**
 * Common Quote Token Mints on Solana Devnet & Mainnet.
 */
export const DEVNET_USDC_MINT = new PublicKey(
  "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr"
);

export const WRAPPED_SOL_MINT = new PublicKey(
  "So11111111111111111111111111111111111111112"
);

/**
 * Default parameters for the Equity Discovery Curve 3-Regime Architecture.
 */
export const DEFAULT_EQUITY_DISCOVERY_REGIMES = {
  // Regime A: Launch & Bootstrapping (Floor to Parity)
  regimeA: {
    priceMultiplierFloor: 0.85, // 85% of anchor price
    priceMultiplierEnd: 1.0,    // 100% of anchor price
    liquidityWeight: 1,        // Baseline depth
  },
  // Regime B: Active Discovery (Concentrated liquidity band around fair value)
  regimeB: {
    priceMultiplierFloor: 1.0,
    priceMultiplierEnd: 1.3,    // 130% of anchor price
    liquidityWeight: 4,        // 4x depth: Low slippage for institutional/retail discovery
  },
  // Regime C: Mature Market (Pre-graduation stability buffer)
  regimeC: {
    priceMultiplierFloor: 1.3,
    priceMultiplierEnd: 1.5,    // 150% of anchor price
    liquidityWeight: 8,        // 8x depth: Smooth buffer before DAMM v2 graduation
  },
};
