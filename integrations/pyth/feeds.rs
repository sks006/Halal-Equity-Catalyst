//! Price feed mappings and registry for Pyth oracles.
//!
//! Replaces the hardcoded-only registry with a dynamic system that:
//! - Boots from hardcoded seeds (so it never starts empty)
//! - Loads a disk cache if present (survives restarts during outages)
//! - Refreshes from Pyth Hermes on a schedule
//! - Records the source of every mapping (seed vs dynamic)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Bootstrap seeds — used ONLY if a dynamic source is unavailable.
// These are NOT authoritative. The dynamic fetch is.
// ─────────────────────────────────────────────────────────────────────────────
pub mod known_feeds {
    pub const SOL_USD: &str = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
    pub const BTC_USD: &str = "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43";
    pub const ETH_USD: &str = "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";
    pub const USDC_USD: &str = "eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a";
    pub const AAPL_USD: &str = "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175084a5ad55688";
    pub const TSLA_USD: &str = "16dad506d7db8da01c87581c87ca897a012a153557d4d578c3b9c9e1bc0632f1";
    pub const MSFT_USD: &str = "d0ca23c1cc005e004ccf1db5bf76aeb6a49218f43dac3d4b275e92de12ded4d1";
    pub const NVDA_USD: &str = "b1073854ed24cbc755dc527418f52b7d271f6cc967bbf8d8129112b18860a593";
    pub const SPY_USD: &str = "19e09bb805456ada3979a7d1cbb4b6d63babc3a0f8e8a9509f68afa5c4c11cd5";
}

// ─────────────────────────────────────────────────────────────────────────────
// Source tracking — lets you know WHERE a mapping came from.
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedSource {
    /// Hardcoded bootstrap value.
    Seed,
    /// Loaded from the on-disk cache.
    DiskCache,
    /// Fetched live from Pyth Hermes.
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedEntry {
    pub feed_id: String,
    pub source: FeedSource,
    /// Unix seconds when this entry was written.
    pub updated_at: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Hermes API response shapes
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Deserialize)]
struct HermesFeed {
    id: String,
    #[serde(default)]
    attributes: FeedAttributes,
}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct FeedAttributes {
    #[serde(default)]
    symbol: String,
    #[serde(default)]
    asset_type: Option<String>,
    #[serde(default)]
    base: Option<String>,
    #[serde(default)]
    quote_currency: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Registry
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug)]
pub struct PythFeedRegistry {
    symbol_to_feed: RwLock<HashMap<String, FeedEntry>>,
    feed_to_symbol: RwLock<HashMap<String, String>>,
    /// Metadata — mostly for diagnostics.
    last_refresh_unix: RwLock<Option<i64>>,
    disk_cache_path: Option<PathBuf>,
}

impl Default for PythFeedRegistry {
    fn default() -> Self {
        Self::new(None)
    }
}

impl PythFeedRegistry {
    /// Create a new registry seeded with hardcoded bootstrap values.
    ///
    /// `disk_cache_path` — optional path to persist/load the dynamic cache.
    pub fn new(disk_cache_path: Option<PathBuf>) -> Self {
        let registry = Self {
            symbol_to_feed: RwLock::new(HashMap::new()),
            feed_to_symbol: RwLock::new(HashMap::new()),
            last_refresh_unix: RwLock::new(None),
            disk_cache_path,
        };

        registry.seed_hardcoded();
        registry
    }

    /// Populate the registry with the hardcoded fallback seeds.
    fn seed_hardcoded(&self) {
        let now = unix_now();
        let mut sym_map = self.symbol_to_feed.write().unwrap();
        let mut feed_map = self.feed_to_symbol.write().unwrap();

        let seeds: &[(&str, &str)] = &[
            // ── Crypto ──
            ("SOL", known_feeds::SOL_USD),
            ("WSOL", known_feeds::SOL_USD),
            ("Crypto.SOL/USD", known_feeds::SOL_USD),
            ("BTC", known_feeds::BTC_USD),
            ("WBTC", known_feeds::BTC_USD),
            ("Crypto.BTC/USD", known_feeds::BTC_USD),
            ("ETH", known_feeds::ETH_USD),
            ("WETH", known_feeds::ETH_USD),
            ("Crypto.ETH/USD", known_feeds::ETH_USD),
            ("USDC", known_feeds::USDC_USD),
            ("Crypto.USDC/USD", known_feeds::USDC_USD),
            // ── Equities ──
            ("AAPL", known_feeds::AAPL_USD),
            ("AAPLX", known_feeds::AAPL_USD),
            ("Equity.US.AAPL/USD", known_feeds::AAPL_USD),
            ("TSLA", known_feeds::TSLA_USD),
            ("TSLAX", known_feeds::TSLA_USD),
            ("Equity.US.TSLA/USD", known_feeds::TSLA_USD),
            ("MSFT", known_feeds::MSFT_USD),
            ("MSFTX", known_feeds::MSFT_USD),
            ("Equity.US.MSFT/USD", known_feeds::MSFT_USD),
            ("NVDA", known_feeds::NVDA_USD),
            ("NVDAX", known_feeds::NVDA_USD),
            ("Equity.US.NVDA/USD", known_feeds::NVDA_USD),
            ("SPY", known_feeds::SPY_USD),
            ("SPYX", known_feeds::SPY_USD),
            ("Equity.US.SPY/USD", known_feeds::SPY_USD),
        ];

        for (symbol, feed_id) in seeds {
            let clean_id = feed_id.trim_start_matches("0x").to_lowercase();
            let upper = symbol.to_uppercase();
            sym_map.insert(
                upper.clone(),
                FeedEntry {
                    feed_id: clean_id.clone(),
                    source: FeedSource::Seed,
                    updated_at: now,
                },
            );
            feed_map.entry(clean_id).or_insert(upper);
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // Public read API — UNCHANGED signature so nothing else breaks.
    // ─────────────────────────────────────────────────────────────────────

    /// Look up feed ID by symbol.
    pub fn get_feed_id(&self, symbol: &str) -> Option<String> {
        let sym_map = self.symbol_to_feed.read().unwrap();
        sym_map
            .get(&symbol.to_uppercase())
            .map(|e| e.feed_id.clone())
    }

    /// Look up full entry (with source + timestamp) by symbol.
    pub fn get_entry(&self, symbol: &str) -> Option<FeedEntry> {
        let sym_map = self.symbol_to_feed.read().unwrap();
        sym_map.get(&symbol.to_uppercase()).cloned()
    }

    /// Look up symbol by feed ID.
    pub fn get_symbol(&self, feed_id: &str) -> Option<String> {
        let clean_id = feed_id.trim_start_matches("0x").to_lowercase();
        let feed_map = self.feed_to_symbol.read().unwrap();
        feed_map.get(&clean_id).cloned()
    }

    /// List all registered symbols.
    pub fn list_symbols(&self) -> Vec<String> {
        let sym_map = self.symbol_to_feed.read().unwrap();
        sym_map.keys().cloned().collect()
    }

    /// Unix seconds of last successful refresh, if any.
    pub fn last_refresh_unix(&self) -> Option<i64> {
        *self.last_refresh_unix.read().unwrap()
    }

    /// Register a custom or override symbol -> feed_id mapping (kept for compat).
    pub fn register_feed(&self, symbol: &str, feed_id: &str) {
        let clean_id = feed_id.trim_start_matches("0x").to_lowercase();
        let upper = symbol.to_uppercase();
        let now = unix_now();

        let mut sym_map = self.symbol_to_feed.write().unwrap();
        let mut feed_map = self.feed_to_symbol.write().unwrap();

        sym_map.insert(
            upper.clone(),
            FeedEntry {
                feed_id: clean_id.clone(),
                source: FeedSource::Seed,
                updated_at: now,
            },
        );
        feed_map.insert(clean_id, upper);
    }

    // ─────────────────────────────────────────────────────────────────────
    // Disk cache
    // ─────────────────────────────────────────────────────────────────────

    /// Load the disk cache into the registry, if a path was configured.
    /// Returns `Ok(true)` if a cache was loaded, `Ok(false)` if there was
    /// nothing to load.
    pub fn load_from_disk(&self) -> Result<bool, std::io::Error> {
        let Some(path) = &self.disk_cache_path else {
            return Ok(false);
        };
        if !path.exists() {
            return Ok(false);
        }

        let raw = std::fs::read_to_string(path)?;
        let cached: HashMap<String, FeedEntry> = serde_json::from_str(&raw)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut sym_map = self.symbol_to_feed.write().unwrap();
        let mut feed_map = self.feed_to_symbol.write().unwrap();
        let mut count = 0;

        for (symbol, mut entry) in cached {
            // Mark provenance so you can tell disk cache from live.
            entry.source = FeedSource::DiskCache;
            feed_map.insert(entry.feed_id.clone(), symbol.clone());
            sym_map.insert(symbol, entry);
            count += 1;
        }

        tracing::info!(entries = count, "Loaded Pyth feed cache from disk");
        Ok(true)
    }

    /// Persist the current dynamic entries to disk.
    fn save_to_disk(&self) -> Result<(), std::io::Error> {
        let Some(path) = &self.disk_cache_path else {
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let sym_map = self.symbol_to_feed.read().unwrap();
        // Only persist dynamic entries — seeds can be re-seeded from code.
        let dynamic: HashMap<_, _> = sym_map
            .iter()
            .filter(|(_, e)| e.source == FeedSource::Dynamic)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let raw = serde_json::to_string_pretty(&dynamic)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, raw)?;
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────
    // Dynamic fetch from Pyth Hermes
    // ─────────────────────────────────────────────────────────────────────

    /// Fetch the full feed list from Hermes and merge it into the registry.
    ///
    /// Uses a temporary map and atomic swap so a partial fetch never leaves
    /// the registry half-populated.
    pub async fn refresh_from_hermes(
        &self,
        client: &reqwest::Client,
        hermes_url: &str,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/v2/price_feeds", hermes_url.trim_end_matches('/'));

        let feeds: Vec<HermesFeed> = client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if feeds.is_empty() {
            return Err("Hermes returned an empty feed list".into());
        }

        let now = unix_now();
        let mut new_sym: HashMap<String, FeedEntry> = HashMap::new();
        let mut new_feed: HashMap<String, String> = HashMap::new();

        for feed in &feeds {
            let clean_id = feed.id.trim_start_matches("0x").to_lowercase();

            // Every alias Pyth gives us + our normalized short tickers.
            let mut aliases: Vec<String> = Vec::new();
            if !feed.attributes.symbol.is_empty() {
                aliases.push(feed.attributes.symbol.to_uppercase());
            }
            if let Some(base) = &feed.attributes.base {
                let b = base.to_uppercase();
                aliases.push(b.clone());
                aliases.push(format!("{}X", b));
            }
            // Parse "Equity.US.NVDA/USD" → "NVDA" and "NVDAX"
            if let Some(base_part) = feed
                .attributes
                .symbol
                .split('/')
                .next()
                .and_then(|s| s.rsplit('.').next())
            {
                let b = base_part.to_uppercase();
                if !b.is_empty() {
                    aliases.push(b.clone());
                    aliases.push(format!("{}X", b));
                }
            }

            for alias in aliases {
                if alias.is_empty() {
                    continue;
                }
                // First write wins — priority goes to the earliest alias (full symbol).
                new_sym.entry(alias.clone()).or_insert_with(|| FeedEntry {
                    feed_id: clean_id.clone(),
                    source: FeedSource::Dynamic,
                    updated_at: now,
                });
                new_feed.entry(clean_id.clone()).or_insert(alias);
            }
        }

        let count = new_sym.len();

        // Atomic swap.
        {
            let mut sym_map = self.symbol_to_feed.write().unwrap();
            let mut feed_map = self.feed_to_symbol.write().unwrap();
            *sym_map = new_sym;
            *feed_map = new_feed;
        }

        *self.last_refresh_unix.write().unwrap() = Some(now);

        // Persist so a restart during outage has data.
        if let Err(e) = self.save_to_disk() {
            tracing::warn!("Failed to persist Pyth feed cache: {}", e);
        }

        tracing::info!(entries = count, "Pyth feed registry refreshed from Hermes");
        Ok(count)
    }

    /// Spawn a background task that refreshes on a fixed interval.
    ///
    /// `interval` — how often to refresh.
    /// `hermes_url` — base URL for the Hermes API.
    pub fn spawn_refresh_task(
        self: Arc<Self>,
        interval: Duration,
        hermes_url: String,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to build HTTP client for Pyth refresh: {}", e);
                    return;
                }
            };

            // Jitter the first refresh so multiple instances don't hit Hermes together.
            let jitter = Duration::from_secs((unix_now() as u64) % interval.as_secs().max(1));
            tokio::time::sleep(jitter).await;

            loop {
                match self.refresh_from_hermes(&client, &hermes_url).await {
                    Ok(n) => tracing::info!(entries = n, "Pyth refresh OK"),
                    Err(e) => tracing::error!("Pyth refresh failed: {}", e),
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_seeds_present() {
        let registry = PythFeedRegistry::new(None);
        assert_eq!(
            registry.get_feed_id("SOL").as_deref(),
            Some(known_feeds::SOL_USD)
        );
        assert_eq!(
            registry.get_feed_id("AAPL").as_deref(),
            Some(known_feeds::AAPL_USD)
        );
        let entry = registry.get_entry("SOL").unwrap();
        assert_eq!(entry.source, FeedSource::Seed);
    }

    #[test]
    fn test_disk_cache_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let cache_file = temp_dir.join(format!("pyth_test_{}.json", unix_now()));

        let registry = PythFeedRegistry::new(Some(cache_file.clone()));

        // Simulate dynamic feed insertion
        {
            let mut sym_map = registry.symbol_to_feed.write().unwrap();
            let mut feed_map = registry.feed_to_symbol.write().unwrap();
            sym_map.insert(
                "TEST_COIN".to_string(),
                FeedEntry {
                    feed_id: "abcdef123456".to_string(),
                    source: FeedSource::Dynamic,
                    updated_at: unix_now(),
                },
            );
            feed_map.insert("abcdef123456".to_string(), "TEST_COIN".to_string());
        }

        registry.save_to_disk().unwrap();

        // Create new registry and load from disk
        let registry_loaded = PythFeedRegistry::new(Some(cache_file.clone()));
        let loaded = registry_loaded.load_from_disk().unwrap();
        assert!(loaded);

        let entry = registry_loaded.get_entry("TEST_COIN").unwrap();
        assert_eq!(entry.feed_id, "abcdef123456");
        assert_eq!(entry.source, FeedSource::DiskCache);

        let _ = std::fs::remove_file(cache_file);
    }
}