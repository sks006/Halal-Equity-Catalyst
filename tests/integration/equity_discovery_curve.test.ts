import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";
import { TokenDecimal, TokenType } from "@meteora-ag/dynamic-bonding-curve-sdk";
import { buildEquityDiscoveryCurve } from "../../sdk/src/meteora/equity-discovery";
import { DEFAULT_EQUITY_DISCOVERY_REGIMES } from "../../sdk/src/meteora/constants";

describe("PHASE 2: Equity Discovery Curve (EDC) Integration Tests", () => {
  const dummyQuoteMint = new PublicKey(
    "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr"
  );

  it("should generate a 3-regime curve conforming to Meteora DBC specification", () => {
    const config = buildEquityDiscoveryCurve({
      anchorPrice: 100.0,
      totalTokenSupply: 1_000_000,
      quoteMint: dummyQuoteMint,
      tokenBaseDecimal: TokenDecimal.SIX,
      tokenQuoteDecimal: TokenDecimal.SIX,
    });

    // 1. Must produce exactly 3 curve segments (4 price checkpoints)
    expect(config.curve).to.have.lengthOf(3);

    // 2. Sqrt start price should correspond to Regime A floor ($85)
    expect(config.sqrtStartPrice).to.not.be.undefined;

    // 3. Liquidity weights must scale monotonically: Regime A < Regime B < Regime C
    const lA = config.curve[0].liquidity;
    const lB = config.curve[1].liquidity;
    const lC = config.curve[2].liquidity;

    // LB / LA should be ~ 4x
    const ratioB_A = lB.muln(100).div(lA).toNumber() / 100;
    expect(ratioB_A).to.be.closeTo(4.0, 0.1);

    // LC / LA should be ~ 8x
    const ratioC_A = lC.muln(100).div(lA).toNumber() / 100;
    expect(ratioC_A).to.be.closeTo(8.0, 0.1);
  });

  it("should configure linear fee scheduler for anti-sniping protection", () => {
    const config = buildEquityDiscoveryCurve({
      anchorPrice: 50.0,
      totalTokenSupply: 500_000,
      quoteMint: dummyQuoteMint,
      feeConfig: {
        startingFeeBps: 300,
        endingFeeBps: 25,
        schedulerDurationSeconds: 43200,
        enableDynamicVolatilityFee: true,
      },
    });

    expect(config.poolFees.baseFee).to.not.be.null;
    // Base fee mode 0 is FeeSchedulerLinear
    expect(config.poolFees.baseFee.baseFeeMode).to.equal(0);
    expect(config.poolFees.dynamicFee).to.not.be.null;
  });

  it("should configure DAMM v2 migration parameters", () => {
    const config = buildEquityDiscoveryCurve({
      anchorPrice: 200.0,
      totalTokenSupply: 2_000_000,
      quoteMint: dummyQuoteMint,
    });

    // Migration option 1 is MET_DAMM_V2
    expect(config.migrationOption).to.equal(1);
    // Migration fee option 6 is Customizable
    expect(config.migrationFeeOption).to.equal(6);
    expect(config.migratedPoolFee).to.not.be.null;
    expect(config.migratedPoolFee?.poolFeeBps).to.equal(100);
  });

  it("should have verified production mainnet pool entries for NVDA, AAPL, and SPYx", () => {
    const {
      VERIFIED_METEORA_POOLS,
      BACKED_NVDA_MINT,
      BACKED_AAPL_MINT,
      BACKED_SPYX_MINT,
      MAINNET_USDC_MINT,
      WRAPPED_SOL_MINT,
      METEORA_NVDA_USDC_POOL,
      METEORA_AAPL_USDC_POOL,
      METEORA_SPYX_USDC_POOL,
    } = require("../../sdk/src/meteora/constants");

    expect(VERIFIED_METEORA_POOLS["NVDA-USDC"]).to.not.be.undefined;
    expect(VERIFIED_METEORA_POOLS["NVDA-USDC"].poolAddress.toBase58()).to.equal(
      METEORA_NVDA_USDC_POOL.toBase58()
    );
    expect(VERIFIED_METEORA_POOLS["NVDA-USDC"].baseMint.toBase58()).to.equal(
      BACKED_NVDA_MINT.toBase58()
    );
    expect(VERIFIED_METEORA_POOLS["NVDA-USDC"].quoteMint.toBase58()).to.equal(
      MAINNET_USDC_MINT.toBase58()
    );

    expect(VERIFIED_METEORA_POOLS["AAPL-USDC"]).to.not.be.undefined;
    expect(VERIFIED_METEORA_POOLS["AAPL-USDC"].poolAddress.toBase58()).to.equal(
      METEORA_AAPL_USDC_POOL.toBase58()
    );
    expect(VERIFIED_METEORA_POOLS["AAPL-USDC"].baseMint.toBase58()).to.equal(
      BACKED_AAPL_MINT.toBase58()
    );

    expect(VERIFIED_METEORA_POOLS["SPYX-USDC"]).to.not.be.undefined;
    expect(VERIFIED_METEORA_POOLS["SPYX-USDC"].poolAddress.toBase58()).to.equal(
      METEORA_SPYX_USDC_POOL.toBase58()
    );
    expect(VERIFIED_METEORA_POOLS["SPYX-USDC"].baseMint.toBase58()).to.equal(
      BACKED_SPYX_MINT.toBase58()
    );

    expect(VERIFIED_METEORA_POOLS["NVDA-SOL"].quoteMint.toBase58()).to.equal(
      WRAPPED_SOL_MINT.toBase58()
    );
  });
});
