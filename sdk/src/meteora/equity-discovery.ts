import {
  ActivationType,
  BaseFeeMode,
  buildCurveWithCustomSqrtPrices,
  CollectFeeMode,
  ConfigParameters,
  createSqrtPrices,
  DammV2BaseFeeMode,
  DammV2DynamicFeeMode,
  MigratedCollectFeeMode,
  MigrationFeeOption,
  MigrationOption,
  TokenAuthorityOption,
  TokenDecimal,
  TokenType,
} from "@meteora-ag/dynamic-bonding-curve-sdk";
import { DEFAULT_EQUITY_DISCOVERY_REGIMES } from "./constants";
import { EquityDiscoveryCurveParams } from "./types";

/**
 * Builds the 3-Regime Equity Discovery Curve tailored specifically for tokenized equity markets.
 *
 * REGIME A: Launch & Bootstrapping (Floor to Parity)
 *  - Low / moderate price sensitivity
 *  - High anti-snipe base fee decaying via FeeScheduler
 *
 * REGIME B: Active Price Discovery (Fair Value Range)
 *  - 4x concentrated liquidity depth around anchor price
 *  - Low slippage for institutional & retail block discovery
 *
 * REGIME C: Mature Market Buffer (Pre-Graduation)
 *  - 8x concentrated liquidity depth
 *  - Volatility dampening and seamless graduation bridge to Meteora DAMM v2
 */
export function buildEquityDiscoveryCurve(
  params: EquityDiscoveryCurveParams
): ConfigParameters {
  const tokenBaseDecimal = params.tokenBaseDecimal ?? TokenDecimal.SIX;
  const tokenQuoteDecimal = params.tokenQuoteDecimal ?? TokenDecimal.SIX;
  const totalTokenSupply = params.totalTokenSupply;
  const anchorPrice = params.anchorPrice;

  const regimes = params.regimes ?? DEFAULT_EQUITY_DISCOVERY_REGIMES;

  // 1. Calculate price checkpoints for the 3 distinct regimes
  const p0 = anchorPrice * regimes.regimeA.priceMultiplierFloor;
  const p1 = anchorPrice * regimes.regimeA.priceMultiplierEnd;
  const p2 = anchorPrice * regimes.regimeB.priceMultiplierEnd;
  const p3 = anchorPrice * regimes.regimeC.priceMultiplierEnd;

  if (p0 <= 0 || p1 <= p0 || p2 <= p1 || p3 <= p2) {
    throw new Error(
      `Invalid regime price progression: P0=${p0}, P1=${p1}, P2=${p2}, P3=${p3}`
    );
  }

  const prices = [p0, p1, p2, p3];
  const sqrtPrices = createSqrtPrices(prices, tokenBaseDecimal, tokenQuoteDecimal);

  // 2. Assign relative liquidity weights for the 3 segments: [WA, WB, WC]
  const liquidityWeights = [
    regimes.regimeA.liquidityWeight,
    regimes.regimeB.liquidityWeight,
    regimes.regimeC.liquidityWeight,
  ];

  // 3. Fee configuration: Anti-sniping scheduler + dynamic volatility protection
  const startingFeeBps = params.feeConfig?.startingFeeBps ?? 250;
  const endingFeeBps = params.feeConfig?.endingFeeBps ?? 50;
  const schedulerDuration = params.feeConfig?.schedulerDurationSeconds ?? 86400; // 24 hours
  const dynamicFeeEnabled = params.feeConfig?.enableDynamicVolatilityFee ?? true;

  // 4. Migration to Meteora DAMM v2 configuration
  const migrationFeePercentage = params.migrationConfig?.migrationFeePercentage ?? 5;
  const creatorFeePercentage = params.migrationConfig?.creatorFeePercentage ?? 50;
  const migratedPoolFeeBps = params.migrationConfig?.migratedPoolFeeBps ?? 100;

  return buildCurveWithCustomSqrtPrices({
    token: {
      tokenType: params.tokenType ?? TokenType.SPLToken,
      tokenBaseDecimal,
      tokenQuoteDecimal,
      tokenAuthorityOption:
        params.tokenAuthorityOption ?? TokenAuthorityOption.PartnerUpdateAuthority,
      totalTokenSupply,
      leftover: 0,
    },
    fee: {
      baseFeeParams: {
        baseFeeMode: BaseFeeMode.FeeSchedulerLinear,
        feeSchedulerParam: {
          startingFeeBps,
          endingFeeBps,
          numberOfPeriod: 60,
          totalDuration: schedulerDuration,
        },
      },
      dynamicFeeEnabled,
      collectFeeMode: CollectFeeMode.QuoteToken,
      creatorTradingFeePercentage: 20, // 20% to creator, 80% to partner/vault
      poolCreationFee: 0,
      enableFirstSwapWithMinFee: false,
    },
    migration: {
      migrationOption: MigrationOption.MET_DAMM_V2,
      migrationFeeOption: MigrationFeeOption.Customizable,
      migrationFee: {
        feePercentage: migrationFeePercentage,
        creatorFeePercentage,
      },
      migratedPoolFee: {
        collectFeeMode: MigratedCollectFeeMode.QuoteToken,
        dynamicFee: DammV2DynamicFeeMode.Enabled,
        poolFeeBps: migratedPoolFeeBps,
        baseFeeMode: DammV2BaseFeeMode.FeeTimeSchedulerLinear,
      },
    },
    liquidityDistribution: {
      partnerLiquidityPercentage: 100, // 100% of migrated LP to partner/vault
      partnerPermanentLockedLiquidityPercentage: 0,
      creatorLiquidityPercentage: 0,
      creatorPermanentLockedLiquidityPercentage: 0,
    },
    lockedVesting: {
      totalLockedVestingAmount: 0,
      numberOfVestingPeriod: 0,
      cliffUnlockAmount: 0,
      totalVestingDuration: 0,
      cliffDurationFromMigrationTime: 0,
    },
    activationType: ActivationType.Timestamp,
    sqrtPrices,
    liquidityWeights,
  });
}
