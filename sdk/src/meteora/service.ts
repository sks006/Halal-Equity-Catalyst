import {
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import { BN } from "bn.js";
import {
  DynamicBondingCurveClient,
  deriveDbcPoolAddress,
  deriveDbcTokenVaultAddress,
} from "@meteora-ag/dynamic-bonding-curve-sdk";
import {
  DBCPoolSummary,
  DBCSwapRequest,
  EquityDiscoveryCurveParams,
} from "./types";
import { buildEquityDiscoveryCurve } from "./equity-discovery";

export class MeteoraDbcService {
  private client: DynamicBondingCurveClient;
  private connection: Connection;

  constructor(connection: Connection, commitment: "confirmed" | "processed" | "finalized" = "confirmed") {
    this.connection = connection;
    this.client = new DynamicBondingCurveClient(connection, commitment);
  }

  /**
   * Get the underlying SDK client.
   */
  public getClient(): DynamicBondingCurveClient {
    return this.client;
  }

  /**
   * Derives the DBC Pool PDA for a token pair and config.
   */
  public derivePoolAddress(
    quoteMint: PublicKey,
    baseMint: PublicKey,
    config: PublicKey
  ): PublicKey {
    return deriveDbcPoolAddress(quoteMint, baseMint, config);
  }

  /**
   * Derives token vault address for the DBC pool.
   */
  public deriveTokenVaultAddress(pool: PublicKey, mint: PublicKey): PublicKey {
    return deriveDbcTokenVaultAddress(pool, mint);
  }

  /**
   * Creates an Equity Discovery Curve config and pool in a single atomic transaction.
   */
  public async createEquityDiscoveryPool(params: {
    payer: Keypair;
    partner: Keypair;
    baseMint: Keypair;
    config: Keypair;
    tokenName: string;
    tokenSymbol: string;
    tokenUri: string;
    curveParams: EquityDiscoveryCurveParams;
  }): Promise<{
    txSignature: string;
    poolAddress: PublicKey;
    configAddress: PublicKey;
    baseMintAddress: PublicKey;
  }> {
    const { payer, partner, baseMint, config, tokenName, tokenSymbol, tokenUri, curveParams } = params;

    // 1. Build the 3-regime curve config
    const curveConfig = buildEquityDiscoveryCurve(curveParams);

    // 2. Build the createConfigAndPool transaction
    const tx = await this.client.partner.createConfigAndPool({
      config: config.publicKey,
      feeClaimer: partner.publicKey,
      leftoverReceiver: partner.publicKey,
      payer: payer.publicKey,
      quoteMint: curveParams.quoteMint,
      ...curveConfig,
      preCreatePoolParam: {
        baseMint: baseMint.publicKey,
        name: tokenName,
        symbol: tokenSymbol,
        uri: tokenUri,
        poolCreator: payer.publicKey,
      },
    });

    tx.feePayer = payer.publicKey;

    // 3. Send and confirm transaction
    const signature = await sendAndConfirmTransaction(this.connection, tx, [
      payer,
      config,
      baseMint,
    ]);

    const poolAddress = this.derivePoolAddress(
      curveParams.quoteMint,
      baseMint.publicKey,
      config.publicKey
    );

    return {
      txSignature: signature,
      poolAddress,
      configAddress: config.publicKey,
      baseMintAddress: baseMint.publicKey,
    };
  }

  /**
   * Executes a swap on the Meteora DBC pool.
   */
  public async executeSwap(
    payer: Keypair,
    request: DBCSwapRequest
  ): Promise<string> {
    const swapTx = await this.client.pool.swap({
      pool: request.pool,
      amountIn: request.amountIn,
      minimumAmountOut: request.minimumAmountOut,
      swapBaseForQuote: request.swapBaseForQuote,
      owner: request.owner,
      payer: request.payer ?? payer.publicKey,
      referralTokenAccount: request.referralTokenAccount ?? null,
    });

    swapTx.feePayer = payer.publicKey;

    return await sendAndConfirmTransaction(this.connection, swapTx, [payer]);
  }

  /**
   * Fetches and summarizes the DBC pool state, reserves, price, and progress.
   */
  public async getPoolSummary(poolAddress: PublicKey): Promise<DBCPoolSummary | null> {
    const pool = await this.client.state.getPool(poolAddress);
    if (!pool) {
      return null;
    }

    const state = pool.poolState;
    const config = await this.client.state.getPoolConfig(state.config);

    const baseReserve = state.baseReserve.toString();
    const quoteReserve = state.quoteReserve.toString();
    const sqrtPrice = state.sqrtPrice.toString();

    // Calculate approximate price and progress
    const progress = await this.client.state.getPoolQuoteTokenCurveProgress(poolAddress);
    const threshold = await this.client.state.getPoolMigrationQuoteThreshold(poolAddress);

    // Q64.64 sqrtPrice to regular price
    const sqrtPriceBigInt = BigInt(state.sqrtPrice.toString());
    const sqrtPriceFloat = Number(sqrtPriceBigInt) / 2 ** 64;
    const currentPrice = sqrtPriceFloat * sqrtPriceFloat;

    return {
      poolAddress: poolAddress.toBase58(),
      configAddress: state.config.toBase58(),
      baseMint: state.baseMint.toBase58(),
      quoteMint: config?.quoteMint ? config.quoteMint.toBase58() : "",
      baseReserve,
      quoteReserve,
      sqrtPrice,
      currentPrice,
      curveProgressPercentage: Number(progress ?? 0),
      migrationThreshold: threshold?.toString() ?? "0",
      isMigrated: Boolean(state.isMigrated),
    };
  }
}
