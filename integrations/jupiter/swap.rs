//! Swap request construction and serialized transaction handling.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use solana_sdk::transaction::VersionedTransaction;

use crate::types::{JupiterError, QuoteResponse, SwapRequest};

/// Constructs a `SwapRequest` payload from a quote and user public key.
pub fn build_swap_request(
    quote: &QuoteResponse,
    user_pubkey: &str,
    prioritization_fee_lamports: Option<u64>,
) -> SwapRequest {
    SwapRequest {
        user_public_key: user_pubkey.to_string(),
        quote_response: quote.clone(),
        wrap_and_unwrap_sol: Some(true),
        use_shared_accounts: Some(true),
        prioritization_fee_lamports,
    }
}

/// Decodes and validates a base64 encoded serialized Solana VersionedTransaction.
pub fn decode_swap_transaction(tx_base64: &str) -> Result<VersionedTransaction, JupiterError> {
    let raw_bytes = BASE64.decode(tx_base64).map_err(|e| {
        JupiterError::InvalidTransaction(format!("Base64 decoding failed for swap tx: {}", e))
    })?;

    bincode::deserialize::<VersionedTransaction>(&raw_bytes).map_err(|e| {
        JupiterError::InvalidTransaction(format!(
            "Failed to deserialize VersionedTransaction: {}",
            e
        ))
    })
}
