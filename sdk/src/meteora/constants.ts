import { PublicKey } from "@solana/web3.js";
import { VerifiedMeteoraPool } from "./types";

/**
 * Meteora Dynamic Bonding Curve (DBC) Program ID for Mainnet and Devnet.
 */
export const METEORA_DBC_PROGRAM_ID = new PublicKey(
  "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"
);

/**
 * Common Quote Token Mints on Solana Devnet & Mainnet.
 */
export const MAINNET_USDC_MINT = new PublicKey(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
);

export const DEVNET_USDC_MINT = new PublicKey(
  "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr"
);

export const WRAPPED_SOL_MINT = new PublicKey(
  "So11111111111111111111111111111111111111112"
);

/**
 * Canonical Backed Tokenized Equity Mints on Solana Mainnet.
 */
export const BACKED_NVDA_MINT = new PublicKey(
  "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh"
);

export const BACKED_AAPL_MINT = new PublicKey(
  "XsbEhLAtcf6HdfpFZ5xEMdqW8nfAvcsP5bdudRLJzJp"
);

export const BACKED_SPYX_MINT = new PublicKey(
  "XsoCS1TfEyfFhfvj8EtZ528L3CaKBDBRqRapnBbDF2W"
);

/**
 * Canonical Meteora DBC Config Account for Equity Discovery Curves.
 */
export const CANONICAL_DBC_CONFIG = new PublicKey(
  "ASv4E2yuTiE5rsWnUG5LWvVYVQ8UnakiYTwgHtz5pf6b"
);

/**
 * Verified Meteora DBC Pool PDAs for Supported Tokenized Equities.
 */
export const METEORA_NVDA_USDC_POOL = new PublicKey(
  "JCqWLp5RAaC3FPFX8MoAt7yRuPqRW2W9ZG3Byigxd1A7"
);

export const METEORA_NVDA_SOL_POOL = new PublicKey(
  "G8RrQbHii2bqRUJg3MvSdkumNU6Kx2xZMHFrefGv3NQY"
);

export const METEORA_AAPL_USDC_POOL = new PublicKey(
  "C3Zm5CTFQxfCdbbDameKXRsmMUz8nfpkHqrevmdX97YS"
);

export const METEORA_AAPL_SOL_POOL = new PublicKey(
  "5Hh5PPeNw65eeJnz9UCzzxEnSrNP7iPrzti5uhVNECKr"
);

export const METEORA_SPYX_USDC_POOL = new PublicKey(
  "CNutHtA6JUuwwWGCcJXobusHdRWZ4EgJTSUxxzXqnRj7"
);

export const METEORA_SPYX_SOL_POOL = new PublicKey(
  "2zF6y56rn6LBeiY6n9Hqk1CVS5o663gSp6r16nDKpPZg"
);

export const METEORA_DEVNET_NVDA_USDC_POOL = new PublicKey(
  "8QByFpYZdnH1jPgL3dQhi7nYrKbYkziiLvWyTaHnE5ff"
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

/**
 * Production verified Meteora DBC pool registry with curve state parameters.
 */
export const VERIFIED_METEORA_POOLS: Record<string, VerifiedMeteoraPool> = {
  "NVDA-USDC": {
    poolAddress: METEORA_NVDA_USDC_POOL,
    configAddress: CANONICAL_DBC_CONFIG,
    baseMint: BACKED_NVDA_MINT,
    quoteMint: MAINNET_USDC_MINT,
    symbol: "NVDAx",
    quoteSymbol: "USDC",
    baseDecimals: 8,
    quoteDecimals: 6,
    initialPriceUsd: 100.73,
    currentPriceUsd: 118.50,
    curveProfile: "equity_discovery",
    graduationThreshold: 100_000.0,
  },
  "NVDA-SOL": {
    poolAddress: METEORA_NVDA_SOL_POOL,
    configAddress: CANONICAL_DBC_CONFIG,
    baseMint: BACKED_NVDA_MINT,
    quoteMint: WRAPPED_SOL_MINT,
    symbol: "NVDAx",
    quoteSymbol: "SOL",
    baseDecimals: 8,
    quoteDecimals: 9,
    initialPriceUsd: 100.73,
    currentPriceUsd: 118.50,
    curveProfile: "equity_discovery",
    graduationThreshold: 750.0,
  },
  "AAPL-USDC": {
    poolAddress: METEORA_AAPL_USDC_POOL,
    configAddress: CANONICAL_DBC_CONFIG,
    baseMint: BACKED_AAPL_MINT,
    quoteMint: MAINNET_USDC_MINT,
    symbol: "AAPLx",
    quoteSymbol: "USDC",
    baseDecimals: 8,
    quoteDecimals: 6,
    initialPriceUsd: 190.50,
    currentPriceUsd: 224.20,
    curveProfile: "equity_discovery",
    graduationThreshold: 100_000.0,
  },
  "AAPL-SOL": {
    poolAddress: METEORA_AAPL_SOL_POOL,
    configAddress: CANONICAL_DBC_CONFIG,
    baseMint: BACKED_AAPL_MINT,
    quoteMint: WRAPPED_SOL_MINT,
    symbol: "AAPLx",
    quoteSymbol: "SOL",
    baseDecimals: 8,
    quoteDecimals: 9,
    initialPriceUsd: 190.50,
    currentPriceUsd: 224.20,
    curveProfile: "equity_discovery",
    graduationThreshold: 750.0,
  },
  "SPYX-USDC": {
    poolAddress: METEORA_SPYX_USDC_POOL,
    configAddress: CANONICAL_DBC_CONFIG,
    baseMint: BACKED_SPYX_MINT,
    quoteMint: MAINNET_USDC_MINT,
    symbol: "SPYx",
    quoteSymbol: "USDC",
    baseDecimals: 8,
    quoteDecimals: 6,
    initialPriceUsd: 480.00,
    currentPriceUsd: 565.00,
    curveProfile: "equity_discovery",
    graduationThreshold: 200_000.0,
  },
  "SPYX-SOL": {
    poolAddress: METEORA_SPYX_SOL_POOL,
    configAddress: CANONICAL_DBC_CONFIG,
    baseMint: BACKED_SPYX_MINT,
    quoteMint: WRAPPED_SOL_MINT,
    symbol: "SPYx",
    quoteSymbol: "SOL",
    baseDecimals: 8,
    quoteDecimals: 9,
    initialPriceUsd: 480.00,
    currentPriceUsd: 565.00,
    curveProfile: "equity_discovery",
    graduationThreshold: 1_500.0,
  },
  "DEVNET-NVDA-USDC": {
    poolAddress: METEORA_DEVNET_NVDA_USDC_POOL,
    configAddress: CANONICAL_DBC_CONFIG,
    baseMint: BACKED_NVDA_MINT,
    quoteMint: DEVNET_USDC_MINT,
    symbol: "NVDAx",
    quoteSymbol: "USDC",
    baseDecimals: 8,
    quoteDecimals: 6,
    initialPriceUsd: 100.73,
    currentPriceUsd: 118.50,
    curveProfile: "equity_discovery",
    graduationThreshold: 100_000.0,
  },
};
