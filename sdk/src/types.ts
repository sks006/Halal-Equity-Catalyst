import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

/** Default on-chain Anchor Program ID for Equity Vault */
export const DEFAULT_PROGRAM_ID = new PublicKey(
  "8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH"
);

/** Standard SPL Token program IDs */
export const TOKEN_PROGRAM_ID = new PublicKey(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
);
export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey(
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);
export const SYSTEM_PROGRAM_ID = new PublicKey(
  "11111111111111111111111111111111"
);

// --- On-Chain Account State Models ---

export interface VaultAccount {
  authority: PublicKey;
  assetMint: PublicKey;
  policy: PublicKey;
  name: string;
  symbol: string;
  totalDeposits: BN;
  totalShares: BN;
  isPaused: boolean;
  bump: number;
  createdAt: BN;
  updatedAt: BN;
}

export interface PolicyAccount {
  vault: PublicKey;
  authority: PublicKey;
  maxLtvBps: number;
  maxPositionBps: number;
  stopLossBps: number;
  takeProfitBps: number;
  rebalanceThresholdBps: number;
  isActive: boolean;
  bump: number;
  updatedAt: BN;
}

export interface UserSharesAccount {
  vault: PublicKey;
  user: PublicKey;
  shares: BN;
  bump: number;
  updatedAt: BN;
}

export interface PositionAccount {
  vault: PublicKey;
  assetMint: PublicKey;
  amount: BN;
  entryPrice: BN;
  currentValue: BN;
  isActive: boolean;
  bump: number;
  createdAt: BN;
  updatedAt: BN;
}

export interface LoanAccount {
  vault: PublicKey;
  borrower: PublicKey;
  collateralMint: PublicKey;
  collateralAmount: BN;
  borrowedAmount: BN;
  ltvBps: number;
  interestRateBps: number;
  isActive: boolean;
  bump: number;
  createdAt: BN;
  updatedAt: BN;
}

export interface ExecutionAccount {
  vault: PublicKey;
  executionId: BN;
  actionType: number;
  status: number;
  inputMint: PublicKey;
  outputMint: PublicKey;
  inputAmount: BN;
  minOutputAmount: BN;
  actualOutputAmount: BN;
  executedAt: BN;
  bump: number;
}

// --- Instruction Parameter Interfaces ---

export interface InitializeVaultParams {
  authority: PublicKey;
  assetMint: PublicKey;
  name: string;
  symbol: string;
  maxLtvBps: number;
  maxPositionBps: number;
}

export interface DepositParams {
  user: PublicKey;
  vault: PublicKey;
  assetMint: PublicKey;
  amount: BN | number | string;
}

export interface WithdrawParams {
  user: PublicKey;
  vault: PublicKey;
  assetMint: PublicKey;
  sharesToBurn: BN | number | string;
}

export interface UpdatePolicyParams {
  authority: PublicKey;
  vault: PublicKey;
  maxLtvBps: number;
  maxPositionBps: number;
  stopLossBps: number;
  takeProfitBps: number;
  rebalanceThresholdBps: number;
  isActive: boolean;
}

export interface TogglePauseParams {
  authority: PublicKey;
  vault: PublicKey;
  isPaused: boolean;
}

export interface ExecuteActionParams {
  authority: PublicKey;
  vault: PublicKey;
  actionType: number;
  executionId: BN | number | string;
  sourceMint: PublicKey;
  targetMint: PublicKey;
  inputAmount: BN | number | string;
  minOutputAmount: BN | number | string;
}

export interface BorrowParams {
  borrower: PublicKey;
  vault: PublicKey;
  borrowAssetMint: PublicKey;
  collateralMint: PublicKey;
  collateralAmount: BN | number | string;
  borrowAmount: BN | number | string;
}

export interface RepayParams {
  borrower: PublicKey;
  vault: PublicKey;
  borrowAssetMint: PublicKey;
  collateralMint: PublicKey;
  repayAmount: BN | number | string;
  collateralToRelease: BN | number | string;
}

// --- Backend REST API Types ---

export interface HealthResponse {
  status: string;
  version: string;
  uptime_seconds: number;
  cluster: string;
}

export interface ReadyResponse {
  ready: boolean;
  database: string;
  redis: string;
  uptime_seconds: number;
}

export interface NormalizedPrice {
  symbol: string;
  feed_id: string;
  price_usd: number;
  price_scaled: number;
  conf_usd: number;
  expo: number;
  publish_time: number;
  is_stale: boolean;
}

export interface QuoteExecutionRequest {
  vault_address: string;
  input_mint: string;
  output_mint: string;
  amount_in: number | string;
  slippage_bps?: number;
  target_symbol?: string;
}

export interface QuoteExecutionVerdict {
  execution_id: string;
  vault_address: string;
  input_mint: string;
  output_mint: string;
  amount_in: number;
  expected_amount_out: number;
  min_amount_out: number;
  price_impact_bps: number;
  price_impact_pct: string;
  approved: boolean;
  rejection_reason?: string | null;
  evaluated_exposure_bps?: number | null;
  is_dry_run: boolean;
  evaluated_at: string;
}

export interface VaultModel {
  vault_address: string;
  authority: string;
  name: string;
  symbol: string;
  deposit_mint: string;
  vault_token_account: string;
  total_shares: number;
  total_deposits: number;
  is_paused: boolean;
  bump: number;
  created_at: string;
  updated_at: string;
}

export interface PolicyModel {
  policy_address: string;
  vault_address: string;
  authority: string;
  max_ltv_bps: number;
  max_position_bps: number;
  stop_loss_bps: number;
  take_profit_bps: number;
  rebalance_threshold_bps: number;
  is_active: boolean;
  bump: number;
  created_at: string;
  updated_at: string;
}

export interface PortfolioModel {
  portfolio_id: string;
  vault_address: string;
  asset_symbol: string;
  asset_mint: string;
  amount: number;
  entry_price_usd: number;
  current_price_usd: number;
  current_value_usd: number;
  target_weight_bps: number;
  current_weight_bps: number;
  last_rebalanced_at?: string | null;
  updated_at: string;
}

export interface EventModel {
  event_id: string;
  vault_address: string;
  event_type: string;
  source: string;
  sentiment_score?: number | null;
  payload: Record<string, unknown>;
  status: string;
  detected_at: string;
  processed_at?: string | null;
}

export interface ExecutionModel {
  execution_id: string;
  vault_address: string;
  event_id?: string | null;
  action: string;
  input_mint: string;
  output_mint: string;
  amount_in: number;
  amount_out_expected: number;
  amount_out_actual?: number | null;
  slippage_bps: number;
  tx_signature?: string | null;
  status: string;
  error_message?: string | null;
  executed_at: string;
  confirmed_at?: string | null;
}

// --- SDK Configuration ---

export interface ClientConfig {
  /** Solana RPC URL (e.g. https://api.devnet.solana.com) */
  rpcUrl?: string;
  /** Backend REST API URL (e.g. http://127.0.0.1:4000) */
  apiUrl?: string;
  /** Override Program ID if non-default */
  programId?: PublicKey;
  /** Custom HTTP headers for API requests */
  headers?: Record<string, string>;
}
