use super::{Config, ConfigError, Environment};

pub trait Rule {
    fn check(&self, c: &Config) -> Result<(), ConfigError>;
}

// ─────────────────────────  Always-on rules  ─────────────────────────

pub struct MaxTradeSizeRule;

impl Rule for MaxTradeSizeRule {
    fn check(&self, c: &Config) -> Result<(), ConfigError> {
        if c.max_trade_size_usd == 0 {
            Err(ConfigError::InvalidMaxTradeSize(0))
        } else {
            Ok(())
        }
    }
}

pub struct SlippageRule;

impl Rule for SlippageRule {
    fn check(&self, c: &Config) -> Result<(), ConfigError> {
        let ceiling = match c.environment {
            Environment::Mainnet => 100,
            _ => 500,
        };
        if c.max_slippage_bps == 0 || c.max_slippage_bps > ceiling {
            Err(ConfigError::InvalidSlippageBps(c.max_slippage_bps, ceiling))
        } else {
            Ok(())
        }
    }
}

pub struct RateLimitRule;

impl Rule for RateLimitRule {
    fn check(&self, c: &Config) -> Result<(), ConfigError> {
        if c.rate_limit_requests_per_minute == 0 {
            Err(ConfigError::InvalidRateLimit(0))
        } else {
            Ok(())
        }
    }
}

// ─────────────────────────  Mainnet-only rules  ─────────────────────

fn mainnet(c: &Config) -> bool {
    c.environment.is_mainnet()
}

pub struct MainnetDatabaseUrlRule;

impl Rule for MainnetDatabaseUrlRule {
    fn check(&self, c: &Config) -> Result<(), ConfigError> {
        if mainnet(c) && c.database_url.trim().is_empty() {
            Err(ConfigError::MissingDatabaseUrl(c.environment))
        } else {
            Ok(())
        }
    }
}

pub struct MainnetAdminApiKeyRule;

impl Rule for MainnetAdminApiKeyRule {
    fn check(&self, c: &Config) -> Result<(), ConfigError> {
        if !mainnet(c) {
            return Ok(());
        }
        let key = c.admin_api_key.trim();
        if key.is_empty() {
            return Err(ConfigError::MissingAdminApiKey(c.environment));
        }
        if key == "catalyst-admin-secret-dev" || key.len() < 16 {
            return Err(ConfigError::DevSecretInProduction(key.to_string()));
        }
        Ok(())
    }
}

pub struct MainnetRpcRule;

impl Rule for MainnetRpcRule {
    fn check(&self, c: &Config) -> Result<(), ConfigError> {
        if !mainnet(c) {
            return Ok(());
        }
        let rpc = c.solana_rpc_url.to_lowercase();
        if rpc.contains("localhost") || rpc.contains("127.0.0.1") || rpc.contains("0.0.0.0") {
            return Err(ConfigError::LocalhostRpcInProduction(
                c.solana_rpc_url.clone(),
            ));
        }
        Ok(())
    }
}

pub struct MainnetSignerRule;

impl Rule for MainnetSignerRule {
    fn check(&self, c: &Config) -> Result<(), ConfigError> {
        if !mainnet(c) {
            return Ok(());
        }
        if c.signer_backend.trim().is_empty() {
            return Err(ConfigError::MissingSignerBackend(c.environment));
        }
        match c.signer_backend.trim().to_lowercase().as_str() {
            "keypair" | "localkeypair" | "file" => {
                if c.execution_signer_path.trim().is_empty() {
                    return Err(ConfigError::MissingSignerPath(c.environment));
                }
            }
            "kms" | "awskms" | "gcpkms" => {
                if c.kms_key_id.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(ConfigError::MissingKmsKeyId(c.environment));
                }
            }
            "hsm" | "pkcs11" | "cloudhsm" => {
                if c.hsm_key_label.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(ConfigError::MissingHsmKeyLabel(c.environment));
                }
            }
            other => return Err(ConfigError::InvalidSignerBackend(other.to_string())),
        }
        Ok(())
    }
}

pub struct MainnetExecutionAuthorityRule;

impl Rule for MainnetExecutionAuthorityRule {
    fn check(&self, c: &Config) -> Result<(), ConfigError> {
        if !mainnet(c) {
            return Ok(());
        }
        if c.expected_execution_authority
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            return Err(ConfigError::MissingExpectedExecutionAuthority(c.environment));
        }
        Ok(())
    }
}

pub struct MainnetPythApiKeyRule;

impl Rule for MainnetPythApiKeyRule {
    fn check(&self, c: &Config) -> Result<(), ConfigError> {
        if !mainnet(c) {
            return Ok(());
        }
        if !c.pyth.is_authenticated() {
            return Err(ConfigError::MissingPythApiKey(c.environment));
        }
        if !c.pyth.hermes_url.starts_with("https://") {
            return Err(ConfigError::InsecurePythUrl(c.pyth.hermes_url.clone()));
        }
        Ok(())
    }
}

// ─────────────────────────  Aggregator  ─────────────────────────

fn default_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(MaxTradeSizeRule),
        Box::new(SlippageRule),
        Box::new(RateLimitRule),
        Box::new(MainnetDatabaseUrlRule),
        Box::new(MainnetAdminApiKeyRule),
        Box::new(MainnetRpcRule),
        Box::new(MainnetSignerRule),
        Box::new(MainnetExecutionAuthorityRule),
        Box::new(MainnetPythApiKeyRule),
    ]
}

pub fn validate(c: &Config) -> Result<(), ConfigError> {
    for rule in default_rules() {
        rule.check(c)?;
    }
    Ok(())
}
