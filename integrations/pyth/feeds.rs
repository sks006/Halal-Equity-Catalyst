//! Price feed mappings and registry for Pyth oracles.

use std::collections::HashMap;
use std::sync::RwLock;

/// Standard Pyth price feed identifiers (Hermes v2 hex strings without 0x prefix).
pub mod known_feeds {
    pub const SOL_USD: &str = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
    pub const BTC_USD: &str = "e62df6e583fca622d5bc837253db582b29e3616fed60fa45071529a215e17f05";
    pub const ETH_USD: &str = "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";
    pub const USDC_USD: &str = "eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a";
    pub const AAPL_USD: &str = "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175ec697df854ab";
    pub const TSLA_USD: &str = "167776b6f68449c25605d8f6356499711202e0766e4a2d829910d54a165a22d7";
    pub const MSFT_USD: &str = "4a985d8868f7004fdb85427d11129994c502b74fa6a2bc3053ba491a98059fa2";
    pub const NVDA_USD: &str = "3155e714652285e6834d8ef0b3558163f4585c5b9679f222956cf57fb3645391";
}

/// Dynamic registry mapping token and equity ticker symbols to Pyth feed IDs.
#[derive(Debug)]
pub struct PythFeedRegistry {
    symbol_to_feed: RwLock<HashMap<String, String>>,
    feed_to_symbol: RwLock<HashMap<String, String>>,
}

impl Default for PythFeedRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PythFeedRegistry {
    pub fn new() -> Self {
        let registry = Self {
            symbol_to_feed: RwLock::new(HashMap::new()),
            feed_to_symbol: RwLock::new(HashMap::new()),
        };

        // Seed standard crypto and token feeds
        registry.register_feed("SOL", known_feeds::SOL_USD);
        registry.register_feed("WSOL", known_feeds::SOL_USD);
        registry.register_feed("BTC", known_feeds::BTC_USD);
        registry.register_feed("WBTC", known_feeds::BTC_USD);
        registry.register_feed("ETH", known_feeds::ETH_USD);
        registry.register_feed("WETH", known_feeds::ETH_USD);
        registry.register_feed("USDC", known_feeds::USDC_USD);

        // Seed synthetic equities
        registry.register_feed("AAPL", known_feeds::AAPL_USD);
        registry.register_feed("TSLA", known_feeds::TSLA_USD);
        registry.register_feed("MSFT", known_feeds::MSFT_USD);
        registry.register_feed("NVDA", known_feeds::NVDA_USD);

        registry
    }

    /// Register a custom or override symbol -> feed_id mapping.
    pub fn register_feed(&self, symbol: &str, feed_id: &str) {
        let clean_feed_id = feed_id.trim_start_matches("0x").to_lowercase();
        let upper_symbol = symbol.to_uppercase();

        let mut sym_map = self.symbol_to_feed.write().unwrap();
        let mut feed_map = self.feed_to_symbol.write().unwrap();

        sym_map.insert(upper_symbol.clone(), clean_feed_id.clone());
        feed_map.insert(clean_feed_id, upper_symbol);
    }

    /// Look up feed ID by symbol.
    pub fn get_feed_id(&self, symbol: &str) -> Option<String> {
        let sym_map = self.symbol_to_feed.read().unwrap();
        sym_map.get(&symbol.to_uppercase()).cloned()
    }

    /// Look up symbol by feed ID.
    pub fn get_symbol(&self, feed_id: &str) -> Option<String> {
        let clean_feed_id = feed_id.trim_start_matches("0x").to_lowercase();
        let feed_map = self.feed_to_symbol.read().unwrap();
        feed_map.get(&clean_feed_id).cloned()
    }

    /// List all registered symbols.
    pub fn list_symbols(&self) -> Vec<String> {
        let sym_map = self.symbol_to_feed.read().unwrap();
        sym_map.keys().cloned().collect()
    }
}
