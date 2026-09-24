//! DEX Quoter abstraction and executable market quote models.
//!
//! # Core Principle
//! Pyth = reference / oracle price.
//! DEX quote = executable market price.
//!
//! This module provides a clean abstraction for querying executable quotes
//! independently of the Pyth oracle network without initiating on-chain swaps
//! or accessing private signing keys.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tracing::{debug, warn};

use crate::client::JupiterClient;
use crate::quotes::parse_price_impact_bps;
use crate::types::{JupiterError, QuoteRequest, QuoteResponse};

/// Errors encountered while querying or validating executable DEX quotes.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DexQuoteError {
    #[error("Invalid trade amount: {0}")]
    InvalidAmount(String),

    #[error("Unsupported token pair: {input_mint} -> {output_mint}")]
    UnsupportedPair {
        input_mint: String,
        output_mint: String,
    },

    #[error("Quote has expired: generated at {quote_timestamp}, expired at {expires_at}, current time {current_time}")]
    ExpiredQuote {
        quote_timestamp: i64,
        expires_at: i64,
        current_time: i64,
    },

    #[error("Price impact {actual_bps} bps exceeds maximum allowable limit of {max_bps} bps")]
    ExcessivePriceImpact { actual_bps: u16, max_bps: u16 },

    #[error("DEX transport or API error: {0}")]
    Transport(String),

    #[error("Internal quoter error: {0}")]
    Internal(String),
}

impl From<JupiterError> for DexQuoteError {
    fn from(err: JupiterError) -> Self {
        match err {
            JupiterError::PriceImpactTooHigh { actual_bps, max_bps } => {
                DexQuoteError::ExcessivePriceImpact { actual_bps, max_bps }
            }
            JupiterError::InvalidQuote(msg) => {
                if msg.to_lowercase().contains("unsupported") || msg.to_lowercase().contains("no mock quote") {
                    DexQuoteError::UnsupportedPair {
                        input_mint: "unknown".to_string(),
                        output_mint: "unknown".to_string(),
                    }
                } else {
                    DexQuoteError::InvalidAmount(msg)
                }
            }
            other => DexQuoteError::Transport(other.to_string()),
        }
    }
}

/// Request parameters for querying an executable DEX quote.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DexQuoteRequest {
    /// Solana SPL token input mint
    pub input_mint: String,
    /// Solana SPL token output mint
    pub output_mint: String,
    /// Input amount in integer base units (lamports/atomic units)
    pub amount_in: u64,
    /// Maximum allowed slippage in basis points (e.g. 50 = 0.50%)
    pub slippage_bps: Option<u16>,
    /// Maximum allowable price impact in basis points before rejecting the quote
    pub max_price_impact_bps: Option<u16>,
    /// Configured quote validity TTL in seconds (default: 15 seconds)
    pub ttl_seconds: Option<u64>,
}

impl DexQuoteRequest {
    /// Constructs a standard executable quote request with default 50 bps slippage and 15s TTL.
    pub fn new(input_mint: impl Into<String>, output_mint: impl Into<String>, amount_in: u64) -> Self {
        Self {
            input_mint: input_mint.into(),
            output_mint: output_mint.into(),
            amount_in,
            slippage_bps: Some(50),
            max_price_impact_bps: Some(100),
            ttl_seconds: Some(15),
        }
    }

    pub fn with_slippage_bps(mut self, bps: u16) -> Self {
        self.slippage_bps = Some(bps);
        self
    }

    pub fn with_max_price_impact_bps(mut self, bps: u16) -> Self {
        self.max_price_impact_bps = Some(bps);
        self
    }

    pub fn with_ttl_seconds(mut self, ttl: u64) -> Self {
        self.ttl_seconds = Some(ttl);
        self
    }
}

/// A single liquidity leg/hop in a DEX routing plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DexRouteStep {
    pub dex_label: String,
    pub amm_key: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub percent: u8,
}

/// Aggregated routing and AMM path information for an executable quote.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DexRouteInfo {
    pub steps: Vec<DexRouteStep>,
    pub num_hops: usize,
    pub primary_dex: String,
}

/// An executable market quote returned by a DEX liquidity provider.
///
/// Contains all pricing, routing, and temporal validity guarantees necessary
/// for safe execution, separated strictly from reference oracle prices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DexQuote {
    /// Unique identifier for this quote instance
    pub quote_id: String,
    /// Provider/DEX identity (e.g. "jupiter", "mock-quoter")
    pub provider_id: String,
    /// Solana SPL token input mint
    pub input_mint: String,
    /// Solana SPL token output mint
    pub output_mint: String,
    /// Input token amount in integer atomic units
    pub input_amount: u64,
    /// Expected output token amount in integer atomic units
    pub expected_output_amount: u64,
    /// Guaranteed minimum output amount given configured slippage tolerance
    pub minimum_output_amount: u64,
    /// Price impact in basis points (1 bp = 0.01%, 100 bps = 1.00%)
    pub price_impact_bps: u16,
    /// Price impact formatted as a percentage string (e.g. "0.05%")
    pub price_impact_pct: String,
    /// Effective rate (`expected_output_amount / input_amount`)
    pub effective_rate: f64,
    /// Unix publication timestamp in seconds
    pub quote_timestamp: i64,
    /// Unix timestamp when this quote expires
    pub expires_at: i64,
    /// Time-to-live duration in seconds
    pub ttl_seconds: u64,
    /// Routing breakdown and intermediate AMMs
    pub route_info: DexRouteInfo,
    /// Raw underlying provider payload (optional for provenance/debugging)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_payload: Option<serde_json::Value>,
}

impl DexQuote {
    /// Evaluates whether the quote is expired relative to the given timestamp.
    pub fn is_expired(&self, current_timestamp: i64) -> bool {
        current_timestamp >= self.expires_at
    }

    /// Validates quote freshness, returning an error if the quote is stale or expired.
    pub fn validate_freshness(&self, current_timestamp: i64) -> Result<(), DexQuoteError> {
        if self.is_expired(current_timestamp) {
            return Err(DexQuoteError::ExpiredQuote {
                quote_timestamp: self.quote_timestamp,
                expires_at: self.expires_at,
                current_time: current_timestamp,
            });
        }
        Ok(())
    }

    /// Validates that price impact does not exceed the allowed threshold.
    pub fn validate_price_impact(&self, max_bps: u16) -> Result<(), DexQuoteError> {
        if self.price_impact_bps > max_bps {
            return Err(DexQuoteError::ExcessivePriceImpact {
                actual_bps: self.price_impact_bps,
                max_bps,
            });
        }
        Ok(())
    }
}

/// Abstract DEX quote provider.
///
/// Implemented by production aggregators (Jupiter) and test harnesses (MockDexQuoter).
/// Contains methods for obtaining executable quotes without signing or executing transactions.
#[async_trait]
pub trait DexQuoter: Send + Sync {
    /// Returns the unique identity string of the DEX provider.
    fn provider_id(&self) -> &str;

    /// Fetches an executable market quote for the specified request parameters.
    ///
    /// Implementations must:
    /// - Reject amounts <= 0.
    /// - Reject unsupported asset pairs.
    /// - Enforce quote expiration and reject excessive price impact.
    async fn get_executable_quote(&self, request: &DexQuoteRequest) -> Result<DexQuote, DexQuoteError>;
}

/// Configurable mock implementation of `DexQuoter` for deterministic unit and integration testing.
#[derive(Clone)]
pub struct MockDexQuoter {
    provider_id: String,
    /// Map of `(input_mint, output_mint)` -> `(rate, price_impact_bps)`
    pairs: Arc<RwLock<HashMap<(String, String), (f64, u16)>>>,
    forced_expired: Arc<RwLock<bool>>,
    fixed_timestamp: Arc<RwLock<Option<i64>>>,
    custom_ttl: Arc<RwLock<Option<u64>>>,
}

impl MockDexQuoter {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            pairs: Arc::new(RwLock::new(HashMap::new())),
            forced_expired: Arc::new(RwLock::new(false)),
            fixed_timestamp: Arc::new(RwLock::new(None)),
            custom_ttl: Arc::new(RwLock::new(None)),
        }
    }

    /// Adds a supported trading pair with a deterministic conversion rate and price impact.
    pub fn add_pair(
        &self,
        input_mint: impl Into<String>,
        output_mint: impl Into<String>,
        rate: f64,
        price_impact_bps: u16,
    ) {
        let mut guard = self.pairs.write().unwrap();
        guard.insert((input_mint.into(), output_mint.into()), (rate, price_impact_bps));
    }

    /// Sets whether returned quotes should be marked as immediately expired.
    pub fn set_forced_expired(&self, expired: bool) {
        *self.forced_expired.write().unwrap() = expired;
    }

    /// Overrides the quote timestamp for deterministic temporal testing.
    pub fn set_fixed_timestamp(&self, ts: Option<i64>) {
        *self.fixed_timestamp.write().unwrap() = ts;
    }

    /// Overrides the default TTL duration in seconds.
    pub fn set_custom_ttl(&self, ttl: Option<u64>) {
        *self.custom_ttl.write().unwrap() = ttl;
    }
}

#[async_trait]
impl DexQuoter for MockDexQuoter {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn get_executable_quote(&self, request: &DexQuoteRequest) -> Result<DexQuote, DexQuoteError> {
        // 1. Validate amount
        if request.amount_in == 0 {
            return Err(DexQuoteError::InvalidAmount(
                "Input amount must be greater than zero".to_string(),
            ));
        }

        // 2. Validate pair support
        let pair_key = (request.input_mint.clone(), request.output_mint.clone());
        let (rate, price_impact_bps) = {
            let guard = self.pairs.read().unwrap();
            *guard.get(&pair_key).ok_or_else(|| DexQuoteError::UnsupportedPair {
                input_mint: request.input_mint.clone(),
                output_mint: request.output_mint.clone(),
            })?
        };

        // 3. Validate price impact policy
        if let Some(max_impact) = request.max_price_impact_bps {
            if price_impact_bps > max_impact {
                return Err(DexQuoteError::ExcessivePriceImpact {
                    actual_bps: price_impact_bps,
                    max_bps: max_impact,
                });
            }
        }

        // 4. Calculate amounts
        let expected_output = (request.amount_in as f64 * rate).round() as u64;
        let slippage = request.slippage_bps.unwrap_or(50);
        let slippage_factor = 1.0 - (slippage as f64 / 10_000.0);
        let minimum_output = (expected_output as f64 * slippage_factor).round() as u64;

        // 5. Timestamps and expiration
        let now = self
            .fixed_timestamp
            .read()
            .unwrap()
            .unwrap_or_else(|| Utc::now().timestamp());

        let ttl = self
            .custom_ttl
            .read()
            .unwrap()
            .or(request.ttl_seconds)
            .unwrap_or(15);

        let expires_at = if *self.forced_expired.read().unwrap() {
            now - 1 // Stale / already expired
        } else {
            now + ttl as i64
        };

        let price_impact_pct = format!("{:.2}%", price_impact_bps as f64 / 100.0);
        let effective_rate = if request.amount_in > 0 {
            expected_output as f64 / request.amount_in as f64
        } else {
            0.0
        };

        let route_info = DexRouteInfo {
            steps: vec![DexRouteStep {
                dex_label: format!("{}-pool", self.provider_id),
                amm_key: format!("amm-key-{}-{}", request.input_mint, request.output_mint),
                input_mint: request.input_mint.clone(),
                output_mint: request.output_mint.clone(),
                in_amount: request.amount_in,
                out_amount: expected_output,
                percent: 100,
            }],
            num_hops: 1,
            primary_dex: self.provider_id.clone(),
        };

        let quote = DexQuote {
            quote_id: format!("quote-{}-{}-{}", self.provider_id, request.amount_in, now),
            provider_id: self.provider_id.clone(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            input_amount: request.amount_in,
            expected_output_amount: expected_output,
            minimum_output_amount: minimum_output,
            price_impact_bps,
            price_impact_pct,
            effective_rate,
            quote_timestamp: now,
            expires_at,
            ttl_seconds: ttl,
            route_info,
            raw_payload: None,
        };

        Ok(quote)
    }
}

#[async_trait]
impl DexQuoter for JupiterClient {
    fn provider_id(&self) -> &str {
        "jupiter"
    }

    async fn get_executable_quote(&self, request: &DexQuoteRequest) -> Result<DexQuote, DexQuoteError> {
        // 1. Validate input amount
        if request.amount_in == 0 {
            return Err(DexQuoteError::InvalidAmount(
                "Input amount must be greater than zero".to_string(),
            ));
        }

        // 2. Build Jupiter request
        let mut jup_req = QuoteRequest::new(
            &request.input_mint,
            &request.output_mint,
            request.amount_in,
        );

        if let Some(slippage) = request.slippage_bps {
            jup_req = jup_req.with_slippage_bps(slippage);
        }

        debug!(
            input_mint = %request.input_mint,
            output_mint = %request.output_mint,
            amount = request.amount_in,
            "Requesting executable DEX quote from Jupiter"
        );

        // 3. Query Jupiter API
        let resp: QuoteResponse = self.get_quote(&jup_req).await?;

        // 4. Parse response values
        let in_amount: u64 = resp.in_amount.parse().map_err(|e| {
            DexQuoteError::Internal(format!("Failed to parse in_amount: {}", e))
        })?;
        let out_amount: u64 = resp.out_amount.parse().map_err(|e| {
            DexQuoteError::Internal(format!("Failed to parse out_amount: {}", e))
        })?;
        let min_amount: u64 = resp.other_amount_threshold.parse().map_err(|e| {
            DexQuoteError::Internal(format!("Failed to parse other_amount_threshold: {}", e))
        })?;

        let price_impact_bps = parse_price_impact_bps(&resp.price_impact_pct)
            .map_err(|e| DexQuoteError::Internal(e.to_string()))?;

        // 5. Enforce price impact limits
        if let Some(max_impact) = request.max_price_impact_bps {
            if price_impact_bps > max_impact {
                warn!(
                    actual_bps = price_impact_bps,
                    max_bps = max_impact,
                    "Executable quote rejected due to excessive price impact"
                );
                return Err(DexQuoteError::ExcessivePriceImpact {
                    actual_bps: price_impact_bps,
                    max_bps: max_impact,
                });
            }
        }

        // 6. Assemble routing steps
        let mut steps = Vec::new();
        let mut primary_dex = "Jupiter".to_string();

        for (i, step) in resp.route_plan.iter().enumerate() {
            let label = step
                .swap_info
                .label
                .clone()
                .unwrap_or_else(|| "AMM".to_string());
            if i == 0 {
                primary_dex = label.clone();
            }

            let step_in: u64 = step.swap_info.in_amount.parse().unwrap_or(0);
            let step_out: u64 = step.swap_info.out_amount.parse().unwrap_or(0);

            steps.push(DexRouteStep {
                dex_label: label,
                amm_key: step.swap_info.amm_key.clone(),
                input_mint: step.swap_info.input_mint.clone(),
                output_mint: step.swap_info.output_mint.clone(),
                in_amount: step_in,
                out_amount: step_out,
                percent: step.percent,
            });
        }

        let num_hops = steps.len().max(1);
        let route_info = DexRouteInfo {
            steps,
            num_hops,
            primary_dex,
        };

        let now = Utc::now().timestamp();
        let ttl = request.ttl_seconds.unwrap_or(15);
        let expires_at = now + ttl as i64;
        let effective_rate = if in_amount > 0 {
            out_amount as f64 / in_amount as f64
        } else {
            0.0
        };

        let raw_payload = serde_json::to_value(&resp).ok();

        Ok(DexQuote {
            quote_id: format!("jup-{}-{}", in_amount, now),
            provider_id: "jupiter".to_string(),
            input_mint: resp.input_mint,
            output_mint: resp.output_mint,
            input_amount: in_amount,
            expected_output_amount: out_amount,
            minimum_output_amount: min_amount,
            price_impact_bps,
            price_impact_pct: format!("{}%", resp.price_impact_pct),
            effective_rate,
            quote_timestamp: now,
            expires_at,
            ttl_seconds: ttl,
            route_info,
            raw_payload,
        })
    }
}
