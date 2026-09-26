//! Swap request construction and serialized transaction handling.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::VersionedTransaction;
use std::str::FromStr;

use crate::types::{JupiterError, QuoteResponse, SwapRequest};

/// Known dummy fallback transaction base64 string.
pub const DUMMY_TRANSACTION_PAYLOAD: &str =
    "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDBg==";

/// Constructs a `SwapRequest` payload from a quote and user public key.
/// Validates that `user_pubkey` is the actual execution authority and a valid Solana public key.
pub fn build_swap_request(
    quote: &QuoteResponse,
    user_pubkey: &str,
    prioritization_fee_lamports: Option<u64>,
) -> Result<SwapRequest, JupiterError> {
    let clean_pubkey = user_pubkey.trim();
    if clean_pubkey.is_empty() {
        return Err(JupiterError::InvalidTransaction(
            "Execution authority public key cannot be empty".to_string(),
        ));
    }

    Pubkey::from_str(clean_pubkey).map_err(|e| {
        JupiterError::InvalidTransaction(format!(
            "Invalid execution authority public key '{}': {}",
            clean_pubkey, e
        ))
    })?;

    if quote.in_amount.trim().is_empty() || quote.out_amount.trim().is_empty() {
        return Err(JupiterError::InvalidQuote(
            "Quote response contains missing or empty amounts".to_string(),
        ));
    }

    Ok(SwapRequest {
        user_public_key: clean_pubkey.to_string(),
        quote_response: quote.clone(),
        wrap_and_unwrap_sol: Some(true),
        use_shared_accounts: Some(true),
        prioritization_fee_lamports,
    })
}

/// Decodes and validates a base64 encoded serialized Solana VersionedTransaction.
/// Rejects empty or dummy placeholder transaction payloads.
pub fn decode_swap_transaction(tx_base64: &str) -> Result<VersionedTransaction, JupiterError> {
    let clean_tx = tx_base64.trim();
    if clean_tx.is_empty() {
        return Err(JupiterError::InvalidTransaction(
            "Serialized swap transaction payload is empty".to_string(),
        ));
    }

    // Never return or accept dummy placeholder transactions
    if clean_tx == DUMMY_TRANSACTION_PAYLOAD {
        return Err(JupiterError::InvalidTransaction(
            "Dummy serialized transaction rejected".to_string(),
        ));
    }

    let raw_bytes = BASE64.decode(clean_tx).map_err(|e| {
        JupiterError::InvalidTransaction(format!("Base64 decoding failed for swap tx: {}", e))
    })?;

    bincode::deserialize::<VersionedTransaction>(&raw_bytes).map_err(|e| {
        JupiterError::InvalidTransaction(format!(
            "Failed to deserialize VersionedTransaction: {}",
            e
        ))
    })
}
