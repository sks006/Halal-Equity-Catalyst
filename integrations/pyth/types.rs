//! Pyth Hermes data transfer types and domain price models.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Pyth error hierarchy.
#[derive(Debug, Error)]
pub enum PythError {
    #[error("HTTP transport error: {0}")]
    Http(String),

    #[error("JSON serialization error: {0}")]
    Json(String),

    #[error("Price feed not found for symbol/id: {0}")]
    FeedNotFound(String),

    #[error("Invalid price data format: {0}")]
    InvalidPriceData(String),

    #[error("Price for {symbol} is stale (published at {publish_time}, max staleness: {max_staleness_secs}s)")]
    StalePrice {
        symbol: String,
        publish_time: i64,
        max_staleness_secs: i64,
    },

    #[error("Confidence interval too wide for {symbol}: price={price}, conf={conf}, ratio={ratio:.4}")]
    ExcessiveConfidenceInterval {
        symbol: String,
        price: f64,
        conf: f64,
        ratio: f64,
    },
}

/// Raw price details returned by Pyth Hermes API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PythRawPrice {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub price: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub conf: String,
    pub expo: i32,
    pub publish_time: i64,
}

/// Helper deserializer to accept both JSON numbers and JSON strings for prices.
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        other => Err(serde::de::Error::custom(format!(
            "Expected string or number, got {:?}",
            other
        ))),
    }
}

/// Parsed price feed entry in Hermes v2 response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedPriceFeed {
    pub id: String,
    pub price: PythRawPrice,
    pub ema_price: Option<PythRawPrice>,
}

/// Outer response envelope returned by `/v2/updates/price/latest`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HermesLatestPriceResponse {
    pub parsed: Option<Vec<ParsedPriceFeed>>,
}

/// Normalized domain price representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedPrice {
    pub symbol: String,
    pub feed_id: String,
    /// Dollar price as a decimal (e.g. 145.23)
    pub price_usd: f64,
    /// Dollar price scaled to micro-USD (6 decimals, e.g. 145.23 -> 145_230_000)
    pub price_scaled: u64,
    /// Confidence interval in USD
    pub conf_usd: f64,
    /// Original Pyth exponent (e.g. -8)
    pub expo: i32,
    /// Unix timestamp of publication in seconds
    pub publish_time: i64,
    /// True if publication timestamp exceeds maximum staleness window
    pub is_stale: bool,
}

impl NormalizedPrice {
    /// Normalizes raw Pyth price details into a domain representation.
    pub fn from_raw(
        symbol: impl Into<String>,
        feed_id: impl Into<String>,
        raw: &PythRawPrice,
        max_staleness_secs: i64,
    ) -> Result<Self, PythError> {
        let symbol_str = symbol.into();
        let feed_id_str = feed_id.into();

        let raw_price: i64 = raw.price.parse().map_err(|e| {
            PythError::InvalidPriceData(format!("Failed to parse raw price '{}': {}", raw.price, e))
        })?;

        let raw_conf: u64 = raw.conf.parse().map_err(|e| {
            PythError::InvalidPriceData(format!("Failed to parse raw conf '{}': {}", raw.conf, e))
        })?;

        if raw_price <= 0 {
            return Err(PythError::InvalidPriceData(format!(
                "Non-positive price for {}: {}",
                symbol_str, raw_price
            )));
        }

        // Pyth price is raw_price * 10^(expo)
        let multiplier = 10f64.powi(raw.expo);
        let price_usd = (raw_price as f64) * multiplier;
        let conf_usd = (raw_conf as f64) * multiplier;

        // Scaled micro-USD: $1.00 = 1_000_000
        let price_scaled = (price_usd * 1_000_000.0).round().max(0.0) as u64;

        let now = Utc::now().timestamp();
        let is_stale = (now - raw.publish_time).abs() > max_staleness_secs;

        Ok(Self {
            symbol: symbol_str,
            feed_id: feed_id_str,
            price_usd,
            price_scaled,
            conf_usd,
            expo: raw.expo,
            publish_time: raw.publish_time,
            is_stale,
        })
    }
}
