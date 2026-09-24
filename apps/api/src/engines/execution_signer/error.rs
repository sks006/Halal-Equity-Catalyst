//! Structured error types for cryptographic signing operations.

use thiserror::Error;

/// Errors arising during cryptographic key loading, signing, and verification.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SignerError {
    #[error("Signer key file not found: {path} (failing closed; no hardcoded fallbacks permitted)")]
    KeyFileNotFound { path: String },

    #[error("Invalid key file format in '{path}': {details}")]
    InvalidKeyFormat { path: String, details: String },

    #[error("External signer ({signer_type}) is unavailable or offline: {reason}")]
    SignerUnavailable {
        signer_type: &'static str,
        reason: String,
    },

    #[error("Cryptographic signing failed: {0}")]
    SigningFailed(String),

    #[error("Production configuration prohibits falling back to development or test keys: {0}")]
    ProductionFallbackProhibited(String),

    #[error("Cryptographic signature verification failed: {0}")]
    VerificationFailed(String),

    #[error("Invalid transaction state: {0}")]
    InvalidTransactionState(String),
}
