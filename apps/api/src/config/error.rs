use super::Environment;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("Missing required database URL in {0} environment")]
    MissingDatabaseUrl(Environment),

    #[error("Missing required administrative API key in {0} environment")]
    MissingAdminApiKey(Environment),

    #[error("Development/test admin secret '{0}' cannot be used in production Mainnet environment")]
    DevSecretInProduction(String),

    #[error("Production Mainnet cannot point to local/localhost RPC endpoint: '{0}'")]
    LocalhostRpcInProduction(String),

    #[error("Missing required signer backend selection in {0} environment (fail closed)")]
    MissingSignerBackend(Environment),

    #[error("Missing execution signer path for keypair backend in {0} environment")]
    MissingSignerPath(Environment),

    #[error("Missing KMS key ID for kms backend in {0} environment")]
    MissingKmsKeyId(Environment),

    #[error("Missing HSM key label for hsm backend in {0} environment")]
    MissingHsmKeyLabel(Environment),

    #[error("Missing expected execution authority public key in {0} environment")]
    MissingExpectedExecutionAuthority(Environment),

    #[error("Invalid signer backend '{0}'. Allowed production backends: 'keypair', 'kms', 'hsm'")]
    InvalidSignerBackend(String),

    #[error("Invalid slippage threshold {0} bps (must be between 1 and {1} bps)")]
    InvalidSlippageBps(u16, u16),

    #[error("Max trade size USD must be greater than zero, got {0}")]
    InvalidMaxTradeSize(u64),

    #[error("Rate limit requests per minute must be greater than zero, got {0}")]
    InvalidRateLimit(u32),

    #[error("Missing required Pyth API key in {0} environment")]
    MissingPythApiKey(Environment),

    #[error("Pyth Hermes URL must use HTTPS in production: '{0}'")]
    InsecurePythUrl(String),
}
