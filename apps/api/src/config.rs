//! Application configuration loaded from environment variables.
//! Strictly separates Development, Testnet/Devnet, and Mainnet environments.

use serde::{Deserialize, Serialize};
use std::env;

/// Distinct operational deployment environments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Testnet,
    Mainnet,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Testnet => write!(f, "testnet"),
            Self::Mainnet => write!(f, "mainnet"),
        }
    }
}

impl Environment {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "mainnet" | "mainnet-beta" | "production" | "prod" => Self::Mainnet,
            "dev" | "local" | "localnet" | "development" => Self::Development,
            _ => Self::Testnet, // Default safe cluster
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub environment: Environment,
    pub api_host: String,
    pub api_port: u16,
    pub solana_rpc_url: String,
    pub solana_fallback_rpc_urls: Vec<String>,
    pub solana_rpc_timeout_ms: u64,
    pub solana_ws_url: String,
    pub solana_cluster: String,
    pub database_url: String,
    pub redis_url: String,
    pub jupiter_api_url: String,
    pub pyth_hermes_url: String,
    pub execution_signer_path: String,
    pub read_only: bool,
    pub max_trade_size_usd: u64,
    pub max_slippage_bps: u16,
    pub admin_api_key: String,
    pub rate_limit_requests_per_minute: u32,
}

impl Config {
    /// Development environment profile (Local validator, test keys).
    pub fn development() -> Self {
        Self {
            environment: Environment::Development,
            api_host: "127.0.0.1".to_string(),
            api_port: 4000,
            solana_rpc_url: "http://127.0.0.1:8899".to_string(),
            solana_fallback_rpc_urls: Vec::new(),
            solana_rpc_timeout_ms: 10_000,
            solana_ws_url: "ws://127.0.0.1:8900".to_string(),
            solana_cluster: "localnet".to_string(),
            database_url: "postgres://postgres:postgres@localhost:5432/equity_catalyst_dev"
                .to_string(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
            jupiter_api_url: "https://quote-api.jup.ag/v6".to_string(),
            pyth_hermes_url: "https://hermes.pyth.network".to_string(),
            execution_signer_path: "~/.config/solana/id.json".to_string(),
            read_only: false,
            max_trade_size_usd: 100_000,
            max_slippage_bps: 100,
            admin_api_key: "catalyst-admin-secret-dev".to_string(),
            rate_limit_requests_per_minute: 120,
        }
    }

    /// Testnet / Devnet environment profile (Solana Devnet, test assets).
    pub fn testnet() -> Self {
        Self {
            environment: Environment::Testnet,
            api_host: "127.0.0.1".to_string(),
            api_port: 4000,
            solana_rpc_url: "https://api.devnet.solana.com".to_string(),
            solana_fallback_rpc_urls: vec![
                "https://devnet.helius-rpc.com/?api-key=public".to_string()
            ],
            solana_rpc_timeout_ms: 15_000,
            solana_ws_url: "wss://api.devnet.solana.com".to_string(),
            solana_cluster: "devnet".to_string(),
            database_url: "postgres://postgres:postgres@localhost:5432/equity_catalyst".to_string(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
            jupiter_api_url: "https://quote-api.jup.ag/v6".to_string(),
            pyth_hermes_url: "https://hermes.pyth.network".to_string(),
            execution_signer_path: "~/.config/solana/id.json".to_string(),
            read_only: true, // Simulation safe default
            max_trade_size_usd: 50_000,
            max_slippage_bps: 50,
            admin_api_key: "catalyst-admin-secret-dev".to_string(),
            rate_limit_requests_per_minute: 120,
        }
    }

    /// Production Mainnet environment profile (Conservative production limits, production RPC).
    pub fn mainnet() -> Self {
        Self {
            environment: Environment::Mainnet,
            api_host: "0.0.0.0".to_string(),
            api_port: 4000,
            solana_rpc_url: env::var("SOLANA_MAINNET_RPC_URL")
                .or_else(|_| env::var("SOLANA_RPC_URL"))
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string()),
            solana_fallback_rpc_urls: {
                let env_fallbacks = env::var("SOLANA_FALLBACK_RPC_URLS")
                    .or_else(|_| env::var("SOLANA_MAINNET_FALLBACK_RPC_URLS"))
                    .unwrap_or_default();
                if env_fallbacks.trim().is_empty() {
                    vec![
                        "https://solana-mainnet.g.alchemy.com/v2/demo".to_string(),
                        "https://rpc.ankr.com/solana".to_string(),
                        "https://api.mainnet-beta.solana.com".to_string(),
                    ]
                } else {
                    env_fallbacks
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
            },
            solana_rpc_timeout_ms: env::var("SOLANA_RPC_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(15_000),
            solana_ws_url: env::var("SOLANA_MAINNET_WS_URL")
                .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com".to_string()),
            solana_cluster: "mainnet-beta".to_string(),
            database_url: env::var("DATABASE_URL").unwrap_or_default(),
            redis_url: env::var("REDIS_URL").unwrap_or_default(),
            jupiter_api_url: "https://quote-api.jup.ag/v6".to_string(),
            pyth_hermes_url: "https://hermes.pyth.network".to_string(),
            execution_signer_path: env::var("EXECUTION_SIGNER_PATH")
                .unwrap_or_else(|_| "/etc/equity-catalyst/signer.json".to_string()),
            read_only: true, // Requires explicit operational override to submit transactions
            max_trade_size_usd: 25_000, // Conservative production limit
            max_slippage_bps: 30, // Strict 30 bps maximum
            admin_api_key: env::var("ADMIN_API_KEY").unwrap_or_default(),
            rate_limit_requests_per_minute: env::var("RATE_LIMIT_PER_MINUTE")
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(60),
        }
    }

    /// Loads configuration from environment variables with profile-based defaults.
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        let env_name = env::var("APP_ENV")
            .or_else(|_| env::var("EQUITY_ENV"))
            .or_else(|_| env::var("SOLANA_CLUSTER"))
            .unwrap_or_else(|_| "devnet".to_string());

        let target_env = Environment::from_str_loose(&env_name);

        let default = match target_env {
            Environment::Development => Self::development(),
            Environment::Testnet => Self::testnet(),
            Environment::Mainnet => Self::mainnet(),
        };

        let api_host = env::var("API_HOST").unwrap_or(default.api_host);
        let api_port = env::var("API_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(default.api_port);
        let solana_rpc_url = env::var("SOLANA_RPC_URL").unwrap_or(default.solana_rpc_url);
        let solana_fallback_rpc_urls = env::var("SOLANA_FALLBACK_RPC_URLS")
            .map(|raw| {
                raw.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<String>>()
            })
            .unwrap_or(default.solana_fallback_rpc_urls);
        let solana_rpc_timeout_ms = env::var("SOLANA_RPC_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(default.solana_rpc_timeout_ms);
        let solana_ws_url = env::var("SOLANA_WS_URL").unwrap_or(default.solana_ws_url);
        let solana_cluster = env::var("SOLANA_CLUSTER").unwrap_or(default.solana_cluster);
        let database_url = env::var("DATABASE_URL").unwrap_or(default.database_url);
        let redis_url = env::var("REDIS_URL").unwrap_or(default.redis_url);
        let jupiter_api_url = env::var("JUPITER_API_URL").unwrap_or(default.jupiter_api_url);
        let pyth_hermes_url = env::var("PYTH_HERMES_URL").unwrap_or(default.pyth_hermes_url);
        let execution_signer_path =
            env::var("EXECUTION_SIGNER_PATH").unwrap_or(default.execution_signer_path);
        let read_only = env::var("EXECUTION_READ_ONLY")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(default.read_only);
        let max_trade_size_usd = env::var("MAX_TRADE_SIZE_USD")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(default.max_trade_size_usd);
        let max_slippage_bps = env::var("MAX_SLIPPAGE_BPS")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(default.max_slippage_bps);
        let admin_api_key = env::var("ADMIN_API_KEY").unwrap_or(default.admin_api_key);
        let rate_limit_requests_per_minute = env::var("RATE_LIMIT_PER_MINUTE")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(default.rate_limit_requests_per_minute);

        Self {
            environment: target_env,
            api_host,
            api_port,
            solana_rpc_url,
            solana_fallback_rpc_urls,
            solana_rpc_timeout_ms,
            solana_ws_url,
            solana_cluster,
            database_url,
            redis_url,
            jupiter_api_url,
            pyth_hermes_url,
            execution_signer_path,
            read_only,
            max_trade_size_usd,
            max_slippage_bps,
            admin_api_key,
            rate_limit_requests_per_minute,
        }
    }

    /// Helper to get socket address formatted string.
    pub fn address(&self) -> String {
        format!("{}:{}", self.api_host, self.api_port)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::testnet()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_separation() {
        let dev = Config::development();
        assert_eq!(dev.environment, Environment::Development);
        assert_eq!(dev.solana_cluster, "localnet");
        assert_eq!(dev.max_slippage_bps, 100);
        assert!(dev.solana_fallback_rpc_urls.is_empty());
        assert_eq!(dev.solana_rpc_timeout_ms, 10_000);

        let test = Config::testnet();
        assert_eq!(test.environment, Environment::Testnet);
        assert_eq!(test.solana_cluster, "devnet");
        assert!(test.read_only);
        assert_eq!(test.max_slippage_bps, 50);
        assert_eq!(test.solana_fallback_rpc_urls.len(), 1);
        assert_eq!(test.solana_rpc_timeout_ms, 15_000);

        let prod = Config::mainnet();
        assert_eq!(prod.environment, Environment::Mainnet);
        assert_eq!(prod.solana_cluster, "mainnet-beta");
        assert!(prod.read_only); // Secure by default
        assert_eq!(prod.max_slippage_bps, 30); // Conservative mainnet bound
        assert_eq!(prod.max_trade_size_usd, 25_000);
        assert!(!prod.solana_fallback_rpc_urls.is_empty());
        assert_eq!(prod.solana_rpc_timeout_ms, 15_000);
    }

    #[test]
    fn test_environment_loose_parsing() {
        assert_eq!(Environment::from_str_loose("mainnet"), Environment::Mainnet);
        assert_eq!(
            Environment::from_str_loose("mainnet-beta"),
            Environment::Mainnet
        );
        assert_eq!(
            Environment::from_str_loose("production"),
            Environment::Mainnet
        );
        assert_eq!(Environment::from_str_loose("dev"), Environment::Development);
        assert_eq!(
            Environment::from_str_loose("localnet"),
            Environment::Development
        );
        assert_eq!(Environment::from_str_loose("devnet"), Environment::Testnet);
        assert_eq!(Environment::from_str_loose("unknown"), Environment::Testnet);
    }
}
