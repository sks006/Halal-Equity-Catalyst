import { Connection, PublicKey } from "@solana/web3.js";

import { EventsClient } from "./events";
import { ExecutionClient } from "./execution";
import { PoliciesClient } from "./policies";
import { PortfolioClient } from "./portfolio";
import {
  ClientConfig,
  DEFAULT_PROGRAM_ID,
  HealthResponse,
  NormalizedPrice,
  ReadyResponse,
} from "./types";
import { VaultsClient } from "./vaults";

/**
 * Main SDK orchestrator client for Equity Catalyst.
 *
 * Provides a unified interface connecting frontend web applications to:
 * 1. Backend REST API services (vault records, normalized Pyth prices, quote evaluation)
 * 2. On-chain Solana Anchor program (PDA derivation, transactions, account states)
 */
export class EquityCatalystClient {
  public readonly connection: Connection;
  public readonly programId: PublicKey;
  public readonly apiUrl?: string;

  public readonly vaults: VaultsClient;
  public readonly policies: PoliciesClient;
  public readonly events: EventsClient;
  public readonly execution: ExecutionClient;
  public readonly portfolio: PortfolioClient;

  constructor(config: ClientConfig = {}) {
    const rpcUrl = config.rpcUrl || "https://api.devnet.solana.com";
    this.connection = new Connection(rpcUrl, "confirmed");
    this.programId = config.programId || DEFAULT_PROGRAM_ID;
    this.apiUrl = config.apiUrl?.replace(/\/+$/, "");

    this.vaults = new VaultsClient(this.connection, this.programId, this.apiUrl);
    this.policies = new PoliciesClient(this.connection, this.programId, this.apiUrl);
    this.events = new EventsClient(this.connection, this.programId, this.apiUrl);
    this.execution = new ExecutionClient(this.connection, this.programId, this.apiUrl);
    this.portfolio = new PortfolioClient(this.connection, this.programId, this.apiUrl);
  }

  /** Factory constructor for Solana Devnet */
  public static forDevnet(
    apiUrl: string = "http://127.0.0.1:4000",
    rpcUrl: string = "https://api.devnet.solana.com"
  ): EquityCatalystClient {
    return new EquityCatalystClient({ apiUrl, rpcUrl });
  }

  /** Factory constructor for local test validator */
  public static forLocalnet(
    apiUrl: string = "http://127.0.0.1:4000",
    rpcUrl: string = "http://127.0.0.1:8899"
  ): EquityCatalystClient {
    return new EquityCatalystClient({ apiUrl, rpcUrl });
  }

  /** Checks API service liveness probe */
  public async getHealth(): Promise<HealthResponse> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }
    const resp = await fetch(`${this.apiUrl}/health`);
    if (!resp.ok) throw new Error(`Health check failed: ${resp.statusText}`);
    const data = await resp.json();
    return data as HealthResponse;
  }

  /** Checks API service readiness probe (Postgres + Redis health) */
  public async getReady(): Promise<ReadyResponse> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }
    const resp = await fetch(`${this.apiUrl}/ready`);
    if (!resp.ok) throw new Error(`Readiness check failed: ${resp.statusText}`);
    const data = await resp.json();
    return data as ReadyResponse;
  }

  /** Fetches normalized oracle price for a symbol via the backend API */
  public async getOraclePrice(symbol: string): Promise<NormalizedPrice> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }
    const resp = await fetch(`${this.apiUrl}/oracle/price/${symbol}`);
    if (!resp.ok) {
      const errText = await resp.text();
      throw new Error(`Failed to fetch oracle price (${resp.status}): ${errText}`);
    }
    const data = await resp.json();
    return data as NormalizedPrice;
  }
}
