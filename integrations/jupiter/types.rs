//! Data transfer types and request/response models for the Jupiter v6 API.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Jupiter error hierarchy.
#[derive(Debug, Error)]
pub enum JupiterError {
    #[error("HTTP transport error: {0}")]
    Http(String),

    #[error("JSON serialization error: {0}")]
    Json(String),

    #[error("Jupiter API error (status {status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Invalid quote response: {0}")]
    InvalidQuote(String),

    #[error("Invalid swap transaction payload: {0}")]
    InvalidTransaction(String),

    #[error("Price impact {actual_bps} bps exceeds maximum limit of {max_bps} bps")]
    PriceImpactTooHigh { actual_bps: u16, max_bps: u16 },
}

/// Request parameters for querying a swap quote from `/v6/quote`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_direct_routes: Option<bool>,
}

impl QuoteRequest {
    pub fn new(input_mint: impl Into<String>, output_mint: impl Into<String>, amount: u64) -> Self {
        Self {
            input_mint: input_mint.into(),
            output_mint: output_mint.into(),
            amount,
            slippage_bps: Some(50), // default 0.50%
            swap_mode: Some("ExactIn".to_string()),
            only_direct_routes: None,
        }
    }

    pub fn with_slippage_bps(mut self, bps: u16) -> Self {
        self.slippage_bps = Some(bps);
        self
    }
}

/// Swap execution info for a single DEX leg in a route plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: Option<String>,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub fee_amount: Option<String>,
    pub fee_mint: Option<String>,
}

/// A route step in Jupiter's routing plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlanStep {
    pub swap_info: SwapInfo,
    pub percent: u8,
}

/// Response returned by the Jupiter `/v6/quote` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub input_mint: String,
    pub in_amount: String,
    pub output_mint: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u16,
    pub price_impact_pct: String,
    pub route_plan: Vec<RoutePlanStep>,
    #[serde(default)]
    pub context_slot: Option<u64>,
    #[serde(default)]
    pub time_taken: Option<f64>,
}

/// Payload sent to the `/v6/swap` endpoint to assemble a serialized transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    pub user_public_key: String,
    pub quote_response: QuoteResponse,
    #[serde(default)]
    pub wrap_and_unwrap_sol: Option<bool>,
    #[serde(default)]
    pub use_shared_accounts: Option<bool>,
    #[serde(default)]
    pub prioritization_fee_lamports: Option<u64>,
}

/// Response returned by the `/v6/swap` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
    #[serde(default)]
    pub prioritization_fee_lamports: Option<u64>,
}
