pub mod accounts;
pub mod anchor_client;
pub mod rpc;
pub mod websocket;

pub use accounts::*;
pub use anchor_client::AnchorClient;
pub use rpc::{
    AccountInfo, SignatureStatus, SolanaRpcClient, TokenAccountBalance,
    TransactionConfirmationStatus,
};
pub use websocket::{AccountNotification, LogsNotification, SolanaWebSocketClient};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolanaError {
    #[error("Account data too short: expected at least {expected} bytes, got {actual}")]
    AccountDataTooShort { expected: usize, actual: usize },

    #[error("Invalid Anchor account discriminator: expected {expected:?}, got {actual:?}")]
    InvalidDiscriminator { expected: [u8; 8], actual: [u8; 8] },

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("RPC transport failure: {0}")]
    RpcTransport(String),

    #[error("Solana RPC error (code {code}): {message}")]
    RpcError { code: i64, message: String },

    #[error("WebSocket transport error: {0}")]
    WebSocketTransport(String),

    #[error("Transaction execution failed on-chain ({signature}): {error}")]
    TransactionExecutionFailed { signature: String, error: String },

    #[error("Transaction confirmation timed out for signature {signature} after {timeout_secs}s")]
    ConfirmationTimeout {
        signature: String,
        timeout_secs: u64,
    },

    #[error("Transaction submission is disabled (Solana client is operating in READ-ONLY mode)")]
    TransactionsDisabled,

    #[error("Invalid public key string: {0}")]
    InvalidPublicKey(String),

    #[error("Unauthorized DEX program ID: {0}. Only whitelisted DEX programs (Jupiter, Meteora) are permitted.")]
    UnauthorizedDexProgram(String),

    #[error("Invalid execution parameters: {0}")]
    InvalidParameters(String),
}
