//! Pyth dynamic subscription models and deterministic subscription sets.
//!
//! Provides the core data structures for tracking active approved asset mappings
//! and feeding them into downstream Pyth oracles and stream managers.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Normalizes a Pyth feed ID into a clean, lowercase 64-character hex string without `0x`.
pub fn normalize_feed_id(feed_id: &str) -> String {
    let trimmed = feed_id.trim();
    let stripped = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        &trimmed[2..]
    } else {
        trimmed
    };
    stripped.trim().to_lowercase()
}

/// Represents an individual active Pyth oracle price feed subscription for an asset.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PythSubscription {
    /// Canonical asset identifier (e.g. "backed:AAPLx", "backed:NVDAx")
    pub asset_id: String,
    /// Human-readable ticker symbol (e.g. "AAPL", "NVDA")
    pub symbol: String,
    /// Underlying SPL token mint address on Solana
    pub mint_address: String,
    /// Pyth price feed hex identifier (normalized, no 0x prefix)
    pub pyth_feed_id: String,
}

impl PythSubscription {
    /// Creates a new `PythSubscription` with normalized feed ID.
    pub fn new(
        asset_id: impl Into<String>,
        symbol: impl Into<String>,
        mint_address: impl Into<String>,
        pyth_feed_id: impl AsRef<str>,
    ) -> Self {
        Self {
            asset_id: asset_id.into(),
            symbol: symbol.into().to_uppercase(),
            mint_address: mint_address.into(),
            pyth_feed_id: normalize_feed_id(pyth_feed_id.as_ref()),
        }
    }
}

impl fmt::Display for PythSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PythSubscription({} [{}]: feed={})",
            self.symbol, self.asset_id, self.pyth_feed_id
        )
    }
}

/// A normalized, deterministically sorted collection of Pyth subscriptions.
///
/// Implements deterministic equality (`PartialEq` / `Eq`) such that two `SubscriptionSet`
/// instances containing the same mappings evaluate as equal regardless of database row
/// ordering or insertion sequence.
#[derive(Debug, Clone, Eq, Serialize, Deserialize, Default)]
pub struct SubscriptionSet {
    subscriptions: Vec<PythSubscription>,
}

impl SubscriptionSet {
    /// Creates a new `SubscriptionSet` from a vector of subscriptions.
    ///
    /// Subscriptions are automatically sorted and deduplicated by canonical `asset_id`
    /// to guarantee deterministic ordering.
    pub fn new(subscriptions: Vec<PythSubscription>) -> Self {
        let mut set = Self { subscriptions };
        set.normalize();
        set
    }

    /// Creates an empty `SubscriptionSet`.
    pub fn empty() -> Self {
        Self {
            subscriptions: Vec::new(),
        }
    }

    /// Normalizes internal subscriptions by sorting deterministically and deduplicating.
    fn normalize(&mut self) {
        // Sort deterministically by (asset_id, pyth_feed_id)
        self.subscriptions.sort();
        // Deduplicate in case identical subscriptions exist
        self.subscriptions.dedup();
    }

    /// Returns the number of subscriptions in this set.
    pub fn len(&self) -> usize {
        self.subscriptions.len()
    }

    /// Returns `true` if this subscription set is empty.
    pub fn is_empty(&self) -> bool {
        self.subscriptions.is_empty()
    }

    /// Borrow the underlying slice of subscriptions.
    pub fn subscriptions(&self) -> &[PythSubscription] {
        &self.subscriptions
    }

    /// Returns the current Pyth feed IDs as a sorted, deduplicated `Vec<String>`.
    pub fn feed_ids(&self) -> Vec<String> {
        let feed_set: BTreeSet<String> = self
            .subscriptions
            .iter()
            .map(|s| s.pyth_feed_id.clone())
            .collect();
        feed_set.into_iter().collect()
    }

    /// Returns all symbols in this subscription set.
    pub fn symbols(&self) -> Vec<String> {
        self.subscriptions
            .iter()
            .map(|s| s.symbol.clone())
            .collect()
    }

    /// Checks if a subscription exists for the given asset ID.
    pub fn contains_asset(&self, asset_id: &str) -> bool {
        self.subscriptions.iter().any(|s| s.asset_id == asset_id)
    }

    /// Checks if a subscription exists for the given symbol (case-insensitive).
    pub fn contains_symbol(&self, symbol: &str) -> bool {
        let upper = symbol.to_uppercase();
        self.subscriptions.iter().any(|s| s.symbol == upper)
    }

    /// Checks if a subscription exists for the given Pyth feed ID.
    pub fn contains_feed(&self, feed_id: &str) -> bool {
        let clean = normalize_feed_id(feed_id);
        self.subscriptions.iter().any(|s| s.pyth_feed_id == clean)
    }

    /// Finds a subscription by canonical asset ID.
    pub fn get_by_asset(&self, asset_id: &str) -> Option<&PythSubscription> {
        self.subscriptions.iter().find(|s| s.asset_id == asset_id)
    }

    /// Finds a subscription by ticker symbol (case-insensitive).
    pub fn get_by_symbol(&self, symbol: &str) -> Option<&PythSubscription> {
        let upper = symbol.to_uppercase();
        self.subscriptions.iter().find(|s| s.symbol == upper)
    }

    /// Finds a subscription by Pyth feed ID.
    pub fn get_by_feed(&self, feed_id: &str) -> Option<&PythSubscription> {
        let clean = normalize_feed_id(feed_id);
        self.subscriptions.iter().find(|s| s.pyth_feed_id == clean)
    }

    /// Enriches a `ParsedPriceFeed` by matching its Pyth feed ID against active approved subscriptions.
    ///
    /// Preserves:
    /// - Pyth feed ID
    /// - Asset ID
    /// - Mint address
    /// - Publish time
    /// - Confidence
    /// - Price
    ///
    /// Does NOT use symbols to infer feed identity.
    /// Returns `None` if the feed ID is NOT present in this approved subscription set.
    pub fn enrich_feed(&self, feed: &crate::types::ParsedPriceFeed) -> Option<EnrichedPriceUpdate> {
        let sub = self.get_by_feed(&feed.id)?;
        Some(EnrichedPriceUpdate {
            pyth_feed_id: sub.pyth_feed_id.clone(),
            asset_id: sub.asset_id.clone(),
            mint_address: sub.mint_address.clone(),
            symbol: sub.symbol.clone(),
            publish_time: feed.price.publish_time,
            confidence: feed.price.conf.clone(),
            price: feed.price.price.clone(),
            expo: feed.price.expo,
            ema_price: feed.ema_price.as_ref().map(|p| p.price.clone()),
            ema_conf: feed.ema_price.as_ref().map(|p| p.conf.clone()),
        })
    }

    /// Enriches all matching feeds from a `PythPriceUpdateEvent`.
    ///
    /// Silently filters out feeds not present in this approved subscription set.
    pub fn enrich_event(
        &self,
        event: &crate::types::PythPriceUpdateEvent,
    ) -> Vec<EnrichedPriceUpdate> {
        event
            .feeds()
            .iter()
            .filter_map(|feed| self.enrich_feed(feed))
            .collect()
    }
}

/// Fully enriched price update preserving canonical asset identity and Pyth market data.
///
/// Preserves:
/// - Pyth feed ID
/// - Asset ID
/// - Mint address
/// - Publish time
/// - Confidence interval
/// - Price mantissa & exponent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnrichedPriceUpdate {
    /// Pyth feed identifier (normalized hex, no 0x prefix)
    pub pyth_feed_id: String,
    /// Canonical asset identifier (e.g. "backed:AAPLx")
    pub asset_id: String,
    /// Underlying SPL token mint address on Solana
    pub mint_address: String,
    /// Ticker symbol for display (e.g. "AAPL")
    pub symbol: String,
    /// Unix publication timestamp reported by Pyth oracle
    pub publish_time: i64,
    /// Raw confidence interval as reported by Pyth
    pub confidence: String,
    /// Raw price mantissa as reported by Pyth
    pub price: String,
    /// Pyth price exponent (e.g. -8)
    pub expo: i32,
    /// Optional EMA price mantissa
    pub ema_price: Option<String>,
    /// Optional EMA confidence interval
    pub ema_conf: Option<String>,
}

impl EnrichedPriceUpdate {
    /// Returns raw price parsed as integer mantissa.
    pub fn parse_price_raw(&self) -> Result<i64, std::num::ParseIntError> {
        self.price.parse()
    }

    /// Returns confidence interval parsed as integer.
    pub fn parse_conf_raw(&self) -> Result<u64, std::num::ParseIntError> {
        self.confidence.parse()
    }

    /// Calculates dollar price as f64 (for display/legacy).
    pub fn price_usd(&self) -> Result<f64, std::num::ParseIntError> {
        let raw = self.parse_price_raw()?;
        Ok((raw as f64) * 10f64.powi(self.expo))
    }

    /// Calculates dollar confidence interval as f64.
    pub fn conf_usd(&self) -> Result<f64, std::num::ParseIntError> {
        let raw = self.parse_conf_raw()?;
        Ok((raw as f64) * 10f64.powi(self.expo))
    }
}

/// Deterministic comparison for `SubscriptionSet`.
///
/// Two subscription sets are equal if and only if they contain the exact same
/// subscriptions, regardless of source ordering.
impl PartialEq for SubscriptionSet {
    fn eq(&self, other: &Self) -> bool {
        if self.subscriptions.len() != other.subscriptions.len() {
            return false;
        }

        let mut a = self.subscriptions.clone();
        let mut b = other.subscriptions.clone();
        a.sort();
        b.sort();
        a == b
    }
}
