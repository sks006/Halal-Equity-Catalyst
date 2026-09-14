import { EquityCatalystClient, VaultModel, PolicyModel, PortfolioModel } from "@equity-catalyst/sdk";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:4000";
const RPC_URL = process.env.NEXT_PUBLIC_SOLANA_RPC_URL || "https://api.devnet.solana.com";

let clientInstance: EquityCatalystClient | null = null;

export function getSdkClient(): EquityCatalystClient {
  if (!clientInstance) {
    clientInstance = new EquityCatalystClient({
      apiUrl: API_URL,
      rpcUrl: RPC_URL,
    });
  }
  return clientInstance;
}

// Fallback demo fixtures for seamless presentation when local devnet/API has no seeded vaults
export const DEMO_VAULTS: VaultModel[] = [
  {
    vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    authority: "auth99X8c1V2b3N4EQTYv7cK89Wq3yK9u4J2b8j9Q1M6",
    name: "Solana Liquid Growth Alpha",
    symbol: "SLGA",
    asset_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
    total_deposits: 4850000000000, // 4,850,000 USDC
    total_shares: 4420000000000,
    is_paused: false,
    created_at: "2026-09-01T12:00:00Z",
  },
  {
    vault_address: "JUP99X8c1V2b3N4EQTYv7cK89Wq3yK9u4J2b8j9Q1M6",
    authority: "auth4J2b8j9Q1M6z9Y7w9X8c1V2b3N4EQTYv7cK89Wq3",
    name: "Jupiter Delta Neutral Yield",
    symbol: "JDNY",
    asset_mint: "So11111111111111111111111111111111111111112", // WSOL
    total_deposits: 1250000000000, // 1,250,000 USDC
    total_shares: 1190000000000,
    is_paused: false,
    created_at: "2026-09-05T08:30:00Z",
  },
  {
    vault_address: "PYTH89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4EQTY",
    authority: "auth9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4EQTYv7cK89Wq",
    name: "Autonomous Pyth Momentum",
    symbol: "APM",
    asset_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    total_deposits: 890000000000, // 890,000 USDC
    total_shares: 820000000000,
    is_paused: false,
    created_at: "2026-09-10T15:45:00Z",
  },
];

export const DEMO_POLICY: PolicyModel = {
  policy_id: "pol-001",
  vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
  max_ltv_bps: 6500, // 65%
  max_position_bps: 2500, // 25%
  stop_loss_bps: 800, // 8%
  take_profit_bps: 2000, // 20%
  rebalance_threshold_bps: 150, // 1.5%
  is_active: true,
  updated_at: "2026-09-12T10:00:00Z",
};

export const DEMO_POSITIONS: PortfolioModel[] = [
  {
    portfolio_id: "pos-sol-01",
    vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    asset_symbol: "SOL",
    asset_mint: "So11111111111111111111111111111111111111112",
    amount: 14200,
    entry_price_usd: 138.5,
    current_price_usd: 152.2,
    current_value_usd: 2161240,
    current_weight_bps: 4456,
    target_weight_bps: 4500,
    updated_at: "2026-09-14T08:00:00Z",
  },
  {
    portfolio_id: "pos-jup-02",
    vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    asset_symbol: "JUP",
    asset_mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
    amount: 850000,
    entry_price_usd: 0.95,
    current_price_usd: 1.12,
    current_value_usd: 952000,
    current_weight_bps: 1962,
    target_weight_bps: 2000,
    updated_at: "2026-09-14T08:00:00Z",
  },
  {
    portfolio_id: "pos-pyth-03",
    vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    asset_symbol: "PYTH",
    asset_mint: "HZ1JovNiDcZvKhEZAYUt9KtNfoUEvmA9LTUpWZxGpump",
    amount: 1200000,
    entry_price_usd: 0.38,
    current_price_usd: 0.44,
    current_value_usd: 528000,
    current_weight_bps: 1088,
    target_weight_bps: 1000,
    updated_at: "2026-09-14T08:00:00Z",
  },
  {
    portfolio_id: "pos-usdc-04",
    vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    asset_symbol: "USDC",
    asset_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    amount: 1208760,
    entry_price_usd: 1.0,
    current_price_usd: 1.0,
    current_value_usd: 1208760,
    current_weight_bps: 2494,
    target_weight_bps: 2500,
    updated_at: "2026-09-14T08:00:00Z",
  },
];
