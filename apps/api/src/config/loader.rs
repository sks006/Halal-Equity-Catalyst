use super::source::EnvSource;
use super::{profiles, Environment, PythConfig};

pub fn resolve_environment(src: &impl EnvSource) -> Environment {
    let raw = src
        .get_first(&["APP_ENV", "EQUITY_ENV", "SOLANA_CLUSTER"])
        .unwrap_or_else(|| "devnet".to_string());
    Environment::from_str_loose(&raw)
}

pub fn from_source(src: &impl EnvSource) -> super::Config {
    let env = resolve_environment(src);
    let mut cfg = match env {
        Environment::Development => profiles::development(),
        Environment::Testnet => profiles::testnet(),
        Environment::Mainnet => profiles::mainnet(),
    };
    overlay(src, &mut cfg);
    cfg
}

fn overlay(src: &impl EnvSource, c: &mut super::Config) {
    c.api_host = src.get_or("API_HOST", c.api_host.clone());

    c.api_port = src
        .get_first(&["PORT", "API_PORT"])
        .and_then(|v| v.parse().ok())
        .unwrap_or(c.api_port);

    c.solana_rpc_url = src.get_or("SOLANA_RPC_URL", c.solana_rpc_url.clone());

    if let Some(raw) = src.get_nonempty("SOLANA_FALLBACK_RPC_URLS") {
        c.solana_fallback_rpc_urls = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    c.solana_rpc_timeout_ms = src
        .get_parsed("SOLANA_RPC_TIMEOUT_MS")
        .unwrap_or(c.solana_rpc_timeout_ms);

    c.solana_ws_url = src.get_or("SOLANA_WS_URL", c.solana_ws_url.clone());
    c.solana_cluster = src.get_or("SOLANA_CLUSTER", c.solana_cluster.clone());
    c.database_url = src.get_or("DATABASE_URL", c.database_url.clone());
    c.redis_url = src.get_or("REDIS_URL", c.redis_url.clone());
    c.jupiter_api_url = src.get_or("JUPITER_API_URL", c.jupiter_api_url.clone());

    // Pyth: rebuild from source so the whole sub-struct is one concern.
    c.pyth = PythConfig::from_source(src);

    c.execution_signer_path =
        src.get_or("EXECUTION_SIGNER_PATH", c.execution_signer_path.clone());

    c.signer_backend = src
        .get_first(&["SIGNER_BACKEND", "EXECUTION_SIGNER_BACKEND"])
        .unwrap_or_else(|| c.signer_backend.clone());

    c.expected_execution_authority = src
        .get_first(&["EXPECTED_EXECUTION_AUTHORITY", "EXECUTION_AUTHORITY"])
        .or_else(|| c.expected_execution_authority.clone());

    c.kms_key_id = src.get_nonempty("KMS_KEY_ID").or_else(|| c.kms_key_id.clone());
    c.kms_endpoint = src
        .get_nonempty("KMS_ENDPOINT")
        .or_else(|| c.kms_endpoint.clone());

    c.hsm_slot = src
        .get_parsed("HSM_SLOT")
        .or(c.hsm_slot);

    c.hsm_key_label = src
        .get_nonempty("HSM_KEY_LABEL")
        .or_else(|| c.hsm_key_label.clone());

    c.preflight_simulation_enabled = src
        .get_parsed("PREFLIGHT_SIMULATION_ENABLED")
        .unwrap_or(c.preflight_simulation_enabled);

    c.read_only = src
        .get_parsed("EXECUTION_READ_ONLY")
        .unwrap_or(c.read_only);

    c.max_trade_size_usd = src
        .get_parsed("MAX_TRADE_SIZE_USD")
        .unwrap_or(c.max_trade_size_usd);

    c.max_slippage_bps = src
        .get_parsed("MAX_SLIPPAGE_BPS")
        .unwrap_or(c.max_slippage_bps);

    c.admin_api_key = src.get_or("ADMIN_API_KEY", c.admin_api_key.clone());

    c.rate_limit_requests_per_minute = src
        .get_parsed("RATE_LIMIT_PER_MINUTE")
        .unwrap_or(c.rate_limit_requests_per_minute);
}
