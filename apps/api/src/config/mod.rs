//! Application configuration.
//!
//! Responsibilities are split by module:
//! - `source`      : abstract env access
//! - `environment` : deployment classification
//! - `error`       : the error taxonomy
//! - `pyth`        : Pyth credentials
//! - `profiles`    : per-environment default values
//! - `loader`      : overlays env on a profile
//! - `validation`  : fail-closed production rules
//! - `sanitize`    : log-safe URL redaction

pub mod environment;
pub mod error;
pub mod loader;
pub mod profiles;
pub mod pyth;
pub mod sanitize;
pub mod source;
pub mod validation;

pub use environment::Environment;
pub use error::ConfigError;
pub use pyth::PythConfig;
pub use sanitize::sanitize_connection_url;
pub use source::{EnvSource, MapEnv, ProcessEnv};

use serde::{Deserialize, Serialize};

/// Fully-resolved runtime configuration.
///
/// Deliberately data-only: no constructors, no env reads, no validation.
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
    pub pyth: PythConfig,
    pub execution_signer_path: String,
    pub signer_backend: String,
    pub expected_execution_authority: Option<String>,
    pub kms_key_id: Option<String>,
    pub kms_endpoint: Option<String>,
    pub hsm_slot: Option<u64>,
    pub hsm_key_label: Option<String>,
    pub preflight_simulation_enabled: bool,
    pub read_only: bool,
    pub max_trade_size_usd: u64,
    pub max_slippage_bps: u16,
    pub admin_api_key: String,
    pub rate_limit_requests_per_minute: u32,
}

impl Config {
    // ── Backward-compatible shims (call sites keep working) ─────────

    pub fn development() -> Self {
        profiles::development()
    }
    pub fn testnet() -> Self {
        profiles::testnet()
    }
    pub fn mainnet() -> Self {
        profiles::mainnet()
    }

    /// Reads config from the process environment.
    pub fn from_env() -> Self {
        if dotenvy::dotenv().is_err() {
            let env_name = std::env::var("APP_ENV")
                .or_else(|_| std::env::var("EQUITY_ENV"))
                .unwrap_or_else(|_| "testnet".to_string());
            let _ = dotenvy::from_filename(format!(".env.{}", env_name))
                .or_else(|_| dotenvy::from_filename(".env.testnet"))
                .or_else(|_| dotenvy::from_filename(".env.development"));
        }
        loader::from_source(&ProcessEnv)
    }

    /// Runs the validation rule chain.
    pub fn validate(&self) -> Result<(), ConfigError> {
        validation::validate(self)
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.api_host, self.api_port)
    }
}

impl Default for Config {
    fn default() -> Self {
        profiles::testnet()
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
        assert!(prod.read_only);
        assert_eq!(prod.max_slippage_bps, 30);
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

    #[test]
    fn test_map_env_overlay() {
        let env = MapEnv::new()
            .with("PORT", "8080")
            .with("API_PORT", "9090")
            .with("SOLANA_RPC_URL", "https://custom.rpc.com");
        let cfg = loader::from_source(&env);
        assert_eq!(cfg.api_port, 8080);
        assert_eq!(cfg.solana_rpc_url, "https://custom.rpc.com");

        let env2 = MapEnv::new().with("API_PORT", "9090");
        let cfg2 = loader::from_source(&env2);
        assert_eq!(cfg2.api_port, 9090);
    }
}
