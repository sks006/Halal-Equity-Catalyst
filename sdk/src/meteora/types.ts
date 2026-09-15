import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import {
  TokenDecimal,
  TokenType,
  TokenAuthorityOption,
  ActivationType,
  CollectFeeMode,
  BaseFeeMode,
  MigrationOption,
  MigrationFeeOption,
} from "@meteora-ag/dynamic-bonding-curve-sdk";

export interface RegimeConfig {
  priceMultiplierFloor: number;
  priceMultiplierEnd: number;
  liquidityWeight: number;
}

export interface EquityDiscoveryCurveParams {
  /** Reference anchor price (e.g. fair value, NAV, or oracle price) in quote units per base token */
  anchorPrice: number;
  /** Total token supply for the asset */
  totalTokenSupply: number;
  /** Decimals for base token (standard 6 for SPL equity tokens) */
  tokenBaseDecimal?: TokenDecimal;
  /** Decimals for quote token (6 for USDC, 9 for SOL) */
  tokenQuoteDecimal?: TokenDecimal;
  /** Quote token mint address (USDC or WSOL) */
  quoteMint: PublicKey;
  /** Token authority option */
  tokenAuthorityOption?: TokenAuthorityOption;
  /** Token type (SPL or Token2022) */
  tokenType?: TokenType;
  /** Custom regimes override (defaults to 3-regime architecture: 1x -> 4x -> 8x liquidity) */
  regimes?: {
    regimeA: RegimeConfig;
    regimeB: RegimeConfig;
    regimeC: RegimeConfig;
  };
  /** Anti-sniping and base fee scheduler settings */
  feeConfig?: {
    startingFeeBps: number;
    endingFeeBps: number;
    schedulerDurationSeconds: number;
    enableDynamicVolatilityFee?: boolean;
  };
  /** Migration settings to Meteora DAMM v2 */
  migrationConfig?: {
    migrationFeePercentage: number;
    creatorFeePercentage: number;
    migratedPoolFeeBps: number;
  };
}

export interface DBCSwapRequest {
  pool: PublicKey;
  amountIn: BN;
  minimumAmountOut: BN;
  swapBaseForQuote: boolean;
  owner: PublicKey;
  payer?: PublicKey;
  referralTokenAccount?: PublicKey | null;
}

export interface DBCPoolSummary {
  poolAddress: string;
  configAddress: string;
  baseMint: string;
  quoteMint: string;
  baseReserve: string;
  quoteReserve: string;
  sqrtPrice: string;
  currentPrice: number;
  curveProgressPercentage: number;
  migrationThreshold: string;
  isMigrated: boolean;
}
