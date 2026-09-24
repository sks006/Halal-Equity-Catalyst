//! In-memory market-data store for canonical assets and Pyth price updates.
//!
//! Provides thread-safe, low-latency in-memory storage of the latest validated
//! market data records indexed primarily by canonical asset identity (`asset_id`).
//!
//! Architectural Invariants:
//! 1. Canonical asset identity (`asset_id`) is the primary storage index.
//! 2. Integer-scaled arithmetic (micro-USD, 6 decimals) is used for financial execution
//!    math to avoid floating point precision loss.
//! 3. Thread-safe concurrency using Tokio `RwLock` without holding locks across async operations.
//! 4. Reactive broadcast channel for notifying application components of incoming price updates.
//! 5. Comprehensive freshness and provenance tracking (publish_time, received_at, slot, staleness).

use chrono::Utc;
use equity_catalyst_pyth::{normalize_feed_id, ParsedPriceFeed, PythSubscription};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};
use tracing::debug;

/// Errors originating from the market data store.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MarketDataError {
    #[error("Invalid price value: {0}")]
    InvalidPrice(String),

    #[error("Invalid confidence interval: {0}")]
    InvalidConfidence(String),

    #[error("Integer scaling math overflow: {0}")]
    MathOverflow(String),

    #[error("Missing identity information: {0}")]
    MissingIdentity(String),

    #[error("Price data format error: {0}")]
    InvalidFormat(String),
}

/// Evaluation of price temporal validity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceFreshness {
    /// Price timestamp is within acceptable staleness threshold.
    Fresh,
    /// Price timestamp exceeds staleness threshold.
    Stale,
    /// Price timestamp is in the future by more than allowable tolerance.
    FutureSkew,
}

/// A comprehensive, validated market price update record indexed by canonical asset identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketPriceUpdate {
    /// Canonical asset identifier (e.g. "backed:AAPLx") - Primary Key
    pub asset_id: String,
    /// Ticker symbol (e.g. "AAPL")
    pub symbol: String,
    /// Underlying SPL token mint address on Solana
    pub mint_address: String,
    /// Pyth Hermes feed identifier (normalized, no 0x prefix)
    pub pyth_feed_id: String,

    /// Scaled integer price in micro-USD ($1.00 = 1,000,000 micro-USD).
    /// Authoritative field for financial execution calculations.
    pub price_scaled: u64,
    /// Raw integer price mantissa reported by Pyth oracle.
    pub raw_price: i64,
    /// Pyth decimal exponent (e.g. -8).
    pub expo: i32,

    /// Scaled confidence interval in micro-USD.
    pub conf_scaled: u64,
    /// Raw confidence interval reported by Pyth oracle.
    pub conf_raw: u64,
    /// Confidence width in basis points relative to price (e.g. 50 = 0.50%).
    pub conf_bps: u16,

    /// Compatible floating point USD price for legacy/display compatibility.
    pub price_usd: f64,
    /// Compatible floating point USD confidence interval.
    pub conf_usd: f64,

    /// Unix timestamp in seconds when Pyth publisher published the price.
    pub publish_time: i64,
    /// Unix timestamp in seconds when this store ingested the update.
    pub received_at: i64,
    /// Elapsed age in seconds since publication `(received_at - publish_time)`.
    pub staleness_age_secs: i64,
    /// Whether this update is deemed stale against the configured threshold.
    pub is_stale: bool,
    /// Optional Solana/Pyth slot number for proof provenance.
    pub slot: Option<u64>,
}

impl MarketPriceUpdate {
    /// Constructs a `MarketPriceUpdate` using pure integer arithmetic for scaling.
    #[allow(clippy::too_many_arguments)]
    pub fn from_raw(
        asset_id: impl Into<String>,
        symbol: impl Into<String>,
        mint_address: impl Into<String>,
        pyth_feed_id: impl AsRef<str>,
        raw_price: i64,
        expo: i32,
        conf_raw: u64,
        publish_time: i64,
        max_staleness_secs: i64,
        slot: Option<u64>,
    ) -> Result<Self, MarketDataError> {
        let asset_id_str = asset_id.into();
        let symbol_str = symbol.into().to_uppercase();
        let mint_str = mint_address.into();
        let feed_id_str = normalize_feed_id(pyth_feed_id.as_ref());

        if asset_id_str.trim().is_empty() {
            return Err(MarketDataError::MissingIdentity(
                "asset_id cannot be empty".to_string(),
            ));
        }
        if mint_str.trim().is_empty() {
            return Err(MarketDataError::MissingIdentity(
                "mint_address cannot be empty".to_string(),
            ));
        }

        if raw_price <= 0 {
            return Err(MarketDataError::InvalidPrice(format!(
                "Price for {} must be positive, got raw {}",
                asset_id_str, raw_price
            )));
        }

        // Integer arithmetic for financial execution scaling (micro-USD, 6 decimals)
        let price_scaled = calculate_scaled_int(raw_price as u64, expo)?;
        let conf_scaled = calculate_scaled_int(conf_raw, expo)?;

        let conf_bps = if price_scaled > 0 {
            let bps = ((conf_scaled as u128 * 10_000) / (price_scaled as u128)) as u64;
            bps.min(10_000) as u16
        } else {
            10_000
        };

        // Preserved floating point fields for display and backwards compatibility
        let multiplier = 10f64.powi(expo);
        let price_usd = (raw_price as f64) * multiplier;
        let conf_usd = (conf_raw as f64) * multiplier;

        let received_at = Utc::now().timestamp();
        let staleness_age_secs = (received_at - publish_time).max(0);
        let is_stale = (received_at - publish_time).abs() > max_staleness_secs;

        Ok(Self {
            asset_id: asset_id_str,
            symbol: symbol_str,
            mint_address: mint_str,
            pyth_feed_id: feed_id_str,
            price_scaled,
            raw_price,
            expo,
            conf_scaled,
            conf_raw,
            conf_bps,
            price_usd,
            conf_usd,
            publish_time,
            received_at,
            staleness_age_secs,
            is_stale,
            slot,
        })
    }

    /// Constructs a `MarketPriceUpdate` from a Pyth `ParsedPriceFeed` and `PythSubscription`.
    pub fn from_parsed_feed(
        feed: &ParsedPriceFeed,
        sub: &PythSubscription,
        max_staleness_secs: i64,
    ) -> Result<Self, MarketDataError> {
        let raw_price: i64 = feed.price.price.parse().map_err(|e| {
            MarketDataError::InvalidFormat(format!("Failed to parse raw price '{}': {}", feed.price.price, e))
        })?;
        let raw_conf: u64 = feed.price.conf.parse().map_err(|e| {
            MarketDataError::InvalidFormat(format!("Failed to parse raw conf '{}': {}", feed.price.conf, e))
        })?;

        Self::from_raw(
            &sub.asset_id,
            &sub.symbol,
            &sub.mint_address,
            &sub.pyth_feed_id,
            raw_price,
            feed.price.expo,
            raw_conf,
            feed.price.publish_time,
            max_staleness_secs,
            None,
        )
    }

    /// Convenience constructor from Pyth subscription and feed using default 30s staleness threshold.
    pub fn from_pyth(
        sub: &PythSubscription,
        feed: &ParsedPriceFeed,
    ) -> Result<Self, MarketDataError> {
        Self::from_parsed_feed(feed, sub, 30)
    }

    /// Evaluates whether this record is currently fresh given a maximum staleness window.
    pub fn is_fresh(&self, max_staleness_secs: i64) -> bool {
        let now = Utc::now().timestamp();
        (now - self.publish_time).abs() <= max_staleness_secs
    }

    /// Returns detailed freshness status.
    pub fn freshness_status(&self, max_staleness_secs: i64) -> PriceFreshness {
        let now = Utc::now().timestamp();
        let diff = now - self.publish_time;
        if diff < -max_staleness_secs {
            PriceFreshness::FutureSkew
        } else if diff > max_staleness_secs {
            PriceFreshness::Stale
        } else {
            PriceFreshness::Fresh
        }
    }
}

/// Converts a raw Pyth unsigned value with exponent `expo` into micro-USD integer scale (`10^6`).
///
/// Employs pure integer arithmetic with half-up rounding.
pub fn calculate_scaled_int(raw: u64, expo: i32) -> Result<u64, MarketDataError> {
    if raw == 0 {
        return Ok(0);
    }

    // Target exponent is 6 (micro-USD: 10^6).
    // Target value = raw * 10^(expo + 6).
    let target_expo = expo + 6;

    if target_expo >= 0 {
        let factor = 10u64
            .checked_pow(target_expo as u32)
            .ok_or_else(|| MarketDataError::MathOverflow("Scaling factor exponent overflow".to_string()))?;
        raw.checked_mul(factor)
            .ok_or_else(|| MarketDataError::MathOverflow("Scaled price multiplication overflow".to_string()))
    } else {
        let divisor = 10u64
            .checked_pow((-target_expo) as u32)
            .ok_or_else(|| MarketDataError::MathOverflow("Scaling divisor exponent overflow".to_string()))?;
        // Half-up rounding: (raw + divisor / 2) / divisor
        let half = divisor / 2;
        let rounded = raw
            .checked_add(half)
            .ok_or_else(|| MarketDataError::MathOverflow("Rounding addition overflow".to_string()))?;
        Ok(rounded / divisor)
    }
}

/// Internal storage state indexed by canonical asset identity.
#[derive(Debug, Default)]
struct MarketDataStoreInner {
    /// Authoritative storage indexed by canonical `asset_id` (Primary Key).
    assets: HashMap<String, MarketPriceUpdate>,
    /// Secondary index: SPL token mint address -> canonical `asset_id`.
    mint_to_asset: HashMap<String, String>,
    /// Secondary index: Pyth Hermes feed ID -> canonical `asset_id`.
    feed_to_asset: HashMap<String, String>,
    /// Secondary index: Ticker symbol (uppercase) -> canonical `asset_id`.
    symbol_to_asset: HashMap<String, String>,
}

impl MarketDataStoreInner {
    fn insert(&mut self, update: MarketPriceUpdate) {
        let asset_id = update.asset_id.clone();
        let mint = update.mint_address.clone();
        let feed = update.pyth_feed_id.clone();
        let symbol = update.symbol.to_uppercase();

        self.mint_to_asset.insert(mint, asset_id.clone());
        self.feed_to_asset.insert(feed, asset_id.clone());
        self.symbol_to_asset.insert(symbol, asset_id.clone());
        self.assets.insert(asset_id, update);
    }

    fn resolve_asset_id(&self, identifier: &str) -> Option<&String> {
        // 1. Direct match on primary key `asset_id`
        if self.assets.contains_key(identifier) {
            return self.assets.get_key_value(identifier).map(|(k, _)| k);
        }

        // 2. Secondary match on `mint_address`
        if let Some(asset_id) = self.mint_to_asset.get(identifier) {
            return Some(asset_id);
        }

        // 3. Secondary match on normalized `pyth_feed_id`
        let clean_feed = normalize_feed_id(identifier);
        if let Some(asset_id) = self.feed_to_asset.get(&clean_feed) {
            return Some(asset_id);
        }

        // 4. Secondary fallback on uppercase symbol
        let upper_sym = identifier.to_uppercase();
        if let Some(asset_id) = self.symbol_to_asset.get(&upper_sym) {
            return Some(asset_id);
        }

        None
    }
}

/// In-memory market data store maintaining real-time validated prices for application components.
#[derive(Clone)]
pub struct MarketDataStore {
    inner: Arc<RwLock<MarketDataStoreInner>>,
    broadcast_tx: broadcast::Sender<MarketPriceUpdate>,
}

impl Default for MarketDataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketDataStore {
    /// Creates a new `MarketDataStore` with standard channel capacity.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Creates a `MarketDataStore` with custom broadcast channel capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (broadcast_tx, _) = broadcast::channel(capacity);
        Self {
            inner: Arc::new(RwLock::new(MarketDataStoreInner::default())),
            broadcast_tx,
        }
    }

    /// Ingests a new or updated `MarketPriceUpdate`.
    ///
    /// Stores the update indexed by canonical `asset_id`, drops locks before
    /// any subsequent operations, and notifies subscribers.
    pub async fn update(&self, update: MarketPriceUpdate) -> Result<(), MarketDataError> {
        let update_clone = {
            let mut guard = self.inner.write().await;
            guard.insert(update.clone());
            update
        }; // Write lock is dropped here before broadcasting

        debug!(
            asset_id = %update_clone.asset_id,
            price_scaled = update_clone.price_scaled,
            "Stored market price update and notifying subscribers"
        );

        // Notify active subscribers; ignore error if no receivers currently listening
        let _ = self.broadcast_tx.send(update_clone);
        Ok(())
    }

    /// Retrieves the latest market price record for an asset.
    ///
    /// Accepts canonical `asset_id` (primary key), SPL token `mint_address`,
    /// `pyth_feed_id`, or ticker `symbol`.
    pub async fn get_latest(&self, identifier: &str) -> Option<MarketPriceUpdate> {
        let guard = self.inner.read().await;
        let asset_id = guard.resolve_asset_id(identifier)?;
        guard.assets.get(asset_id).cloned()
    }

    /// Retrieves the latest record strictly by canonical `asset_id`.
    pub async fn get_by_asset_id(&self, asset_id: &str) -> Option<MarketPriceUpdate> {
        let guard = self.inner.read().await;
        guard.assets.get(asset_id).cloned()
    }

    /// Retrieves the latest record strictly by SPL token mint address.
    pub async fn get_by_mint(&self, mint_address: &str) -> Option<MarketPriceUpdate> {
        let guard = self.inner.read().await;
        let asset_id = guard.mint_to_asset.get(mint_address)?;
        guard.assets.get(asset_id).cloned()
    }

    /// Retrieves the latest record strictly by Pyth feed ID.
    pub async fn get_by_feed(&self, feed_id: &str) -> Option<MarketPriceUpdate> {
        let clean = normalize_feed_id(feed_id);
        let guard = self.inner.read().await;
        let asset_id = guard.feed_to_asset.get(&clean)?;
        guard.assets.get(asset_id).cloned()
    }

    /// Retrieves the latest record strictly by ticker symbol.
    pub async fn get_by_symbol(&self, symbol: &str) -> Option<MarketPriceUpdate> {
        let upper = symbol.to_uppercase();
        let guard = self.inner.read().await;
        let asset_id = guard.symbol_to_asset.get(&upper)?;
        guard.assets.get(asset_id).cloned()
    }

    /// Retrieves all current market data records sorted by canonical `asset_id`.
    pub async fn get_all(&self) -> Vec<MarketPriceUpdate> {
        let guard = self.inner.read().await;
        let mut list: Vec<MarketPriceUpdate> = guard.assets.values().cloned().collect();
        list.sort_by(|a, b| a.asset_id.cmp(&b.asset_id));
        list
    }

    /// Returns the total number of tracked canonical assets.
    pub async fn len(&self) -> usize {
        let guard = self.inner.read().await;
        guard.assets.len()
    }

    /// Returns `true` if the store contains no price records.
    pub async fn is_empty(&self) -> bool {
        let guard = self.inner.read().await;
        guard.assets.is_empty()
    }

    /// Checks whether a record exists for the given canonical `asset_id`.
    pub async fn contains_asset(&self, asset_id: &str) -> bool {
        let guard = self.inner.read().await;
        guard.assets.contains_key(asset_id)
    }

    /// Subscribes to real-time price updates emitted by this store.
    pub fn subscribe(&self) -> broadcast::Receiver<MarketPriceUpdate> {
        self.broadcast_tx.subscribe()
    }
}
