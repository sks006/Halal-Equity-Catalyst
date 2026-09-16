use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, warn};

use crate::{
    feeds::PythFeedRegistry,
    types::{HermesLatestPriceResponse, NormalizedPrice, ParsedPriceFeed, PythError, PythRawPrice},
};

/// Client for interacting with the Pyth Network Hermes HTTP API.
#[derive(Clone)]
pub struct PythClient {
    base_url: String,
    http_client: reqwest::Client,
    registry: Arc<PythFeedRegistry>,
    mock_mode: Arc<RwLock<bool>>,
    mock_prices: Arc<RwLock<HashMap<String, PythRawPrice>>>,
}

impl PythClient {
    /// Creates a new PythClient with the specified Hermes endpoint.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            registry: Arc::new(PythFeedRegistry::new()),
            mock_mode: Arc::new(RwLock::new(false)),
            mock_prices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new PythClient in mock mode for testing.
    pub fn new_mock() -> Self {
        let client = Self::new("https://mock-hermes.pyth.network");
        *client.mock_mode.write().unwrap() = true;
        client
    }

    /// Sets mock mode flag.
    pub fn set_mock_mode(&self, enabled: bool) {
        *self.mock_mode.write().unwrap() = enabled;
    }

    /// Injects a mock price for a feed ID or symbol.
    pub fn set_mock_price(
        &self,
        identifier: &str,
        price: &str,
        conf: &str,
        expo: i32,
        publish_time: i64,
    ) {
        let feed_id = if let Some(mapped) = self.registry.get_feed_id(identifier) {
            mapped
        } else {
            identifier.trim_start_matches("0x").to_lowercase()
        };

        let raw = PythRawPrice {
            price: price.to_string(),
            conf: conf.to_string(),
            expo,
            publish_time,
        };

        let mut mocks = self.mock_prices.write().unwrap();
        mocks.insert(feed_id, raw);
    }

    /// Access the internal feed registry.
    pub fn registry(&self) -> &PythFeedRegistry {
        &self.registry
    }

    /// Fetches the latest price for a specific feed ID.
    pub async fn get_latest_price(&self, feed_id: &str) -> Result<ParsedPriceFeed, PythError> {
        let clean_id = feed_id.trim_start_matches("0x").to_lowercase();

        if *self.mock_mode.read().unwrap() {
            let mocks = self.mock_prices.read().unwrap();
            if let Some(price) = mocks.get(&clean_id) {
                return Ok(ParsedPriceFeed {
                    id: clean_id,
                    price: price.clone(),
                    ema_price: Some(price.clone()),
                });
            }
            return Err(PythError::FeedNotFound(clean_id));
        }

        let feeds = self.get_latest_prices(&[&clean_id]).await?;
        feeds
            .into_iter()
            .find(|f| f.id.to_lowercase() == clean_id)
            .ok_or(PythError::FeedNotFound(clean_id))
    }

    /// Fetches latest prices for multiple feed IDs in a single batch query.
    pub async fn get_latest_prices(
        &self,
        feed_ids: &[&str],
    ) -> Result<Vec<ParsedPriceFeed>, PythError> {
        if feed_ids.is_empty() {
            return Ok(Vec::new());
        }

        if *self.mock_mode.read().unwrap() {
            let mocks = self.mock_prices.read().unwrap();
            let mut results = Vec::new();
            for id in feed_ids {
                let clean = id.trim_start_matches("0x").to_lowercase();
                if let Some(price) = mocks.get(&clean) {
                    results.push(ParsedPriceFeed {
                        id: clean,
                        price: price.clone(),
                        ema_price: Some(price.clone()),
                    });
                }
            }
            return Ok(results);
        }

        // Endpoint: /v2/updates/price/latest?ids[]=id1&ids[]=id2&parsed=true
        let mut url = format!("{}/v2/updates/price/latest?parsed=true", self.base_url);
        for id in feed_ids {
            let clean = id.trim_start_matches("0x").to_lowercase();
            url.push_str(&format!("&ids[]={}", clean));
        }

        debug!(url = %url, "Fetching Pyth Hermes price feeds");

        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| PythError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(status = %status, body = %body, "Pyth Hermes request failed");
            return Err(PythError::Http(format!(
                "Hermes API returned status {}: {}",
                status, body
            )));
        }

        let envelope: HermesLatestPriceResponse = resp
            .json()
            .await
            .map_err(|e| PythError::Json(e.to_string()))?;

        Ok(envelope.parsed.unwrap_or_default())
    }

    /// Fetches price feed by token ticker symbol.
    pub async fn get_price_by_symbol(&self, symbol: &str) -> Result<ParsedPriceFeed, PythError> {
        let feed_id = self.registry.get_feed_id(symbol).ok_or_else(|| {
            PythError::FeedNotFound(format!("Symbol '{}' not in registry", symbol))
        })?;

        self.get_latest_price(&feed_id).await
    }

    /// Fetches and normalizes a token price into domain USD representation.
    pub async fn get_normalized_price_by_symbol(
        &self,
        symbol: &str,
        max_staleness_secs: i64,
    ) -> Result<NormalizedPrice, PythError> {
        let feed = self.get_price_by_symbol(symbol).await?;
        NormalizedPrice::from_raw(symbol, &feed.id, &feed.price, max_staleness_secs)
    }
}
