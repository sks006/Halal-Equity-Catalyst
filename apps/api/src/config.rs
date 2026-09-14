//! Application configuration loaded from environment variables.

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_host: String,
    pub api_port: u16,
    pub solana_rpc_url: String,
    pub solana_ws_url: String,
    pub solana_cluster: String,
    pub database_url: String,
    pub redis_url: String,
    pub jupiter_api_url: String,
    pub pyth_hermes_url: String,
    pub execution_signer_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_host: "127.0.0.1".to_string(),
            api_port: 4000,
            solana_rpc_url: "https://api.devnet.solana.com".to_string(),
            solana_ws_url: "wss://api.devnet.solana.com".to_string(),
            solana_cluster: "devnet".to_string(),
            database_url: "postgres://postgres:postgres@localhost:5432/equity_catalyst".to_string(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
            jupiter_api_url: "https://quote-api.jup.ag/v6".to_string(),
            pyth_hermes_url: "https://hermes.pyth.network".to_string(),
            execution_signer_path: "~/.config/solana/id.json".to_string(),
        }
    }
}

impl Config {
    /// Loads configuration from environment variables with optional .env support.
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        let default = Self::default();

        let api_host = env::var("API_HOST").unwrap_or(default.api_host);
        let api_port = env::var("API_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(default.api_port);
        let solana_rpc_url = env::var("SOLANA_RPC_URL").unwrap_or(default.solana_rpc_url);
        let solana_ws_url = env::var("SOLANA_WS_URL").unwrap_or(default.solana_ws_url);
        let solana_cluster = env::var("SOLANA_CLUSTER").unwrap_or(default.solana_cluster);
        let database_url = env::var("DATABASE_URL").unwrap_or(default.database_url);
        let redis_url = env::var("REDIS_URL").unwrap_or(default.redis_url);
        let jupiter_api_url = env::var("JUPITER_API_URL").unwrap_or(default.jupiter_api_url);
        let pyth_hermes_url = env::var("PYTH_HERMES_URL").unwrap_or(default.pyth_hermes_url);
        let execution_signer_path =
            env::var("EXECUTION_SIGNER_PATH").unwrap_or(default.execution_signer_path);

        Self {
            api_host,
            api_port,
            solana_rpc_url,
            solana_ws_url,
            solana_cluster,
            database_url,
            redis_url,
            jupiter_api_url,
            pyth_hermes_url,
            execution_signer_path,
        }
    }

    /// Helper to get socket address formatted string.
    pub fn address(&self) -> String {
        format!("{}:{}", self.api_host, self.api_port)
    }
}
