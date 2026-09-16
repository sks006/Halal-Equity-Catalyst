/**
 * Fully-Typed Frontend API Client for Equity Catalyst.
 *
 * Connects Next.js web application securely to the backend REST services.
 * Invariant: Never contains or leaks signer credentials, private keys, or seed phrases.
 */

export interface ComponentHealth {
  name: string;
  status: "healthy" | "degraded" | "unhealthy";
  message: string;
  latency_ms: number;
}

export interface SystemHealthReport {
  status: "healthy" | "degraded" | "unhealthy";
  timestamp: string;
  components: ComponentHealth[];
}

export interface NormalizedPrice {
  symbol: string;
  price_usd: number;
  confidence_usd: number;
  publish_time: number;
  is_stale: boolean;
}

export interface VerifiedAsset {
  name: string;
  symbol: string;
  issuer: string;
  regulatory_framework: string;
  mint: string;
  decimals: number;
  supported_quote_mints: string[];
  liquidity_venues: string[];
  hackathon_allowed: boolean;
  is_invented_token: boolean;
}

export interface DbcPoolModel {
  pool_address: string;
  config_address: string;
  base_mint: string;
  quote_mint: string;
  token_name: string;
  token_symbol: string;
  tx_signature: string;
  creator: string;
  initial_price_usd: number;
  current_price_usd: number;
  curve_progress_pct: number;
  is_migrated: boolean;
  creation_timestamp: string;
  created_at: string;
  updated_at: string;
}

export interface CreateDbcPoolRequest {
  pool_address: string;
  config_address: string;
  base_mint: string;
  quote_mint: string;
  token_name: string;
  token_symbol: string;
  tx_signature: string;
  creator: string;
  initial_price_usd?: number;
  current_price_usd?: number;
  curve_progress_pct?: number;
  is_migrated?: boolean;
}

export interface VaultModel {
  vault_address: string;
  authority: string;
  name: string;
  symbol: string;
  deposit_mint: string;
  vault_token_account: string;
  total_deposits: number;
  total_shares: number;
  is_paused: boolean;
  bump: number;
  created_at: string;
  updated_at: string;
}

export interface CreateVaultRequest {
  vault_address: string;
  authority: string;
  name: string;
  symbol: string;
  deposit_mint: string;
  vault_token_account: string;
  bump: number;
}

export interface PolicyModel {
  vault_address: string;
  max_position_bps: number;
  target_equity_bps: number;
  min_cash_bps: number;
  rebalance_threshold_bps: number;
  max_drawdown_bps: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateOrUpdatePolicyRequest {
  vault_address: string;
  max_position_bps: number;
  target_equity_bps: number;
  min_cash_bps: number;
  rebalance_threshold_bps: number;
  max_drawdown_bps: number;
  is_active: boolean;
}

export interface PolicyEventModel {
  event_id: string;
  vault_address: string;
  event_type: string;
  source: string;
  status: "pending" | "processing" | "completed" | "failed";
  payload: Record<string, unknown>;
  retry_count: number;
  created_at: string;
  processed_at?: string;
  error_message?: string;
}

export interface CreateEventRequest {
  vault_address: string;
  event_type: string;
  source: string;
  payload: Record<string, unknown>;
}

export interface ExecutionModel {
  execution_id: string;
  vault_address: string;
  event_id?: string;
  action: string;
  input_mint: string;
  output_mint: string;
  amount_in: number;
  amount_out_expected: number;
  amount_out_actual?: number;
  slippage_bps: number;
  tx_signature?: string;
  status: "requested" | "validated" | "simulated" | "submitted" | "confirmed" | "failed";
  error_message?: string;
  executed_at: string;
  confirmed_at?: string;
}

export interface QuoteEvaluationRequest {
  vault_address: string;
  input_mint: string;
  output_mint: string;
  in_amount: number;
  out_amount: number;
  price_impact_pct: number;
  slippage_bps: number;
}

export interface QuoteEvaluationResponse {
  allowed: boolean;
  reason: string;
  price_impact_bps: number;
  max_allowed_impact_bps: number;
}

export interface DbcSimulationInput {
  target_raise_sol: number;
  initial_market_cap_usd: number;
  migration_market_cap_usd: number;
  fee_bps: number;
  quote_sol_price_usd?: number;
}

export interface DbcSimulationResult {
  initial_token_price_usd: number;
  migration_token_price_usd: number;
  tokens_for_curve: number;
  tokens_for_migration: number;
  curve_shape: string;
  sol_reserve_at_migration: number;
  price_progression: Array<{ step_pct: number; price_usd: number; sol_raised: number }>;
}

export class ApiClientError extends Error {
  constructor(
    public readonly status: number,
    public readonly endpoint: string,
    message: string
  ) {
    super(`API Error [${status}] at ${endpoint}: ${message}`);
    this.name = "ApiClientError";
  }
}

export class ApiClient {
  private readonly baseUrl: string;

  constructor(baseUrl?: string) {
    this.baseUrl = (baseUrl || process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:4000").replace(
      /\/+$/,
      ""
    );
  }

  private async request<T>(endpoint: string, options?: RequestInit): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    try {
      const resp = await fetch(url, {
        ...options,
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
          ...options?.headers,
        },
      });

      if (!resp.ok) {
        let errorMsg = resp.statusText;
        try {
          const errJson = await resp.json();
          errorMsg = errJson.message || errJson.error || JSON.stringify(errJson);
        } catch {
          // Response body was not JSON
        }
        throw new ApiClientError(resp.status, endpoint, errorMsg);
      }

      return (await resp.json()) as T;
    } catch (err) {
      if (err instanceof ApiClientError) {
        throw err;
      }
      throw new ApiClientError(0, endpoint, (err as Error).message || "Network request failed");
    }
  }

  // --- Health & System ---
  public async getHealth(): Promise<{ status: string }> {
    return this.request<{ status: string }>("/health");
  }

  public async getDetailedHealth(): Promise<SystemHealthReport> {
    return this.request<SystemHealthReport>("/health/detailed");
  }

  public async getReady(): Promise<{ status: string; postgres: boolean; redis: boolean }> {
    return this.request<{ status: string; postgres: boolean; redis: boolean }>("/ready");
  }

  // --- Oracle / Market Data ---
  public async getOraclePrice(symbol: string): Promise<NormalizedPrice> {
    return this.request<NormalizedPrice>(`/oracle/price/${encodeURIComponent(symbol)}`);
  }

  public async getAllPrices(symbols: string[] = ["NVDA", "AAPL", "MSFT", "TSLA", "SPYx"]): Promise<Record<string, NormalizedPrice>> {
    const results: Record<string, NormalizedPrice> = {};
    await Promise.allSettled(
      symbols.map(async (s) => {
        try {
          const p = await this.getOraclePrice(s);
          results[s] = p;
        } catch {
          // Allow partial price feed degradation gracefully
        }
      })
    );
    return results;
  }

  // --- Verified Assets ---
  public async getVerifiedAssets(): Promise<VerifiedAsset[]> {
    return this.request<VerifiedAsset[]>("/dbc/assets/verified");
  }

  // --- DBC Bonding Curve Pools ---
  public async listDbcPools(): Promise<DbcPoolModel[]> {
    return this.request<DbcPoolModel[]>("/dbc/pools");
  }

  public async getDbcPool(address: string): Promise<DbcPoolModel> {
    return this.request<DbcPoolModel>(`/dbc/pools/${encodeURIComponent(address)}`);
  }

  public async recordDbcPool(payload: CreateDbcPoolRequest): Promise<DbcPoolModel> {
    return this.request<DbcPoolModel>("/dbc/pools", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  public async simulateDbc(input: DbcSimulationInput): Promise<DbcSimulationResult> {
    return this.request<DbcSimulationResult>("/dbc/simulate", {
      method: "POST",
      body: JSON.stringify(input),
    });
  }

  // --- Vaults ---
  public async listVaults(): Promise<VaultModel[]> {
    return this.request<VaultModel[]>("/vaults");
  }

  public async getVault(address: string): Promise<VaultModel> {
    return this.request<VaultModel>(`/vaults/${encodeURIComponent(address)}`);
  }

  public async createVault(payload: CreateVaultRequest): Promise<VaultModel> {
    return this.request<VaultModel>("/vaults", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  // --- Policies ---
  public async listPolicies(): Promise<PolicyModel[]> {
    return this.request<PolicyModel[]>("/policies");
  }

  public async getVaultPolicy(vaultAddress: string): Promise<PolicyModel> {
    return this.request<PolicyModel>(`/policies/${encodeURIComponent(vaultAddress)}`);
  }

  public async createOrUpdatePolicy(payload: CreateOrUpdatePolicyRequest): Promise<PolicyModel> {
    return this.request<PolicyModel>("/policies", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  // --- Events & AI Proposals ---
  public async listEvents(): Promise<PolicyEventModel[]> {
    return this.request<PolicyEventModel[]>("/events");
  }

  public async listPendingEvents(): Promise<PolicyEventModel[]> {
    return this.request<PolicyEventModel[]>("/events/pending");
  }

  public async getVaultEvents(vaultAddress: string): Promise<PolicyEventModel[]> {
    return this.request<PolicyEventModel[]>(`/vaults/${encodeURIComponent(vaultAddress)}/events`);
  }

  public async createEvent(payload: CreateEventRequest): Promise<PolicyEventModel> {
    return this.request<PolicyEventModel>("/events", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  // --- Executions ---
  public async listExecutions(): Promise<ExecutionModel[]> {
    return this.request<ExecutionModel[]>("/executions");
  }

  public async getVaultExecutions(vaultAddress: string): Promise<ExecutionModel[]> {
    return this.request<ExecutionModel[]>(`/vaults/${encodeURIComponent(vaultAddress)}/executions`);
  }

  // --- Quotes ---
  public async evaluateQuote(payload: QuoteEvaluationRequest): Promise<QuoteEvaluationResponse> {
    return this.request<QuoteEvaluationResponse>("/quotes/evaluate", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }
}

// Default singleton instance
let defaultClient: ApiClient | null = null;

export function getApiClient(): ApiClient {
  if (!defaultClient) {
    defaultClient = new ApiClient();
  }
  return defaultClient;
}
