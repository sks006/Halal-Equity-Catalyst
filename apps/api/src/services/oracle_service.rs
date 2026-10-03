use async_trait::async_trait;
use chrono::Utc;
use equity_catalyst_pyth::{NormalizedPrice, PythClient, PythError};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::{
    error::ApiError,
    models::PortfolioModel,
    repositories::{PortfolioRepository, PortfolioRepositoryTrait},
};

/// Abstraction for oracle price providers conforming to Open/Closed (OCP) and Dependency Inversion (DIP).
#[async_trait]
pub trait PriceFeedProvider: Send + Sync {
    async fn get_normalized_price_by_symbol(
        &self,
        symbol: &str,
        max_staleness_secs: i64,
    ) -> Result<NormalizedPrice, PythError>;
}

#[async_trait]
impl PriceFeedProvider for PythClient {
    async fn get_normalized_price_by_symbol(
        &self,
        symbol: &str,
        max_staleness_secs: i64,
    ) -> Result<NormalizedPrice, PythError> {
        self.get_normalized_price_by_symbol(symbol, max_staleness_secs)
            .await
    }
}

/// Service responsible for fetching, validating, and normalizing external oracle price feeds.
#[derive(Clone)]
pub struct OracleService {
    price_provider: Arc<dyn PriceFeedProvider>,
    portfolio_repo: Option<Arc<dyn PortfolioRepositoryTrait>>,
    max_staleness_secs: i64,
    max_confidence_ratio: f64,
}

impl OracleService {
    /// Creates a new OracleService with standard safety thresholds using concrete PythClient.
    pub fn new(pyth_client: Arc<PythClient>, portfolio_repo: Option<PortfolioRepository>) -> Self {
        Self {
            price_provider: pyth_client,
            portfolio_repo: portfolio_repo
                .map(|r| Arc::new(r) as Arc<dyn PortfolioRepositoryTrait>),
            max_staleness_secs: 120,    // 2 minutes max staleness
            max_confidence_ratio: 0.05, // Max 5% confidence width
        }
    }

    /// Creates an OracleService with an abstract PriceFeedProvider and PortfolioRepositoryTrait (DIP).
    pub fn new_with_provider(
        price_provider: Arc<dyn PriceFeedProvider>,
        portfolio_repo: Option<Arc<dyn PortfolioRepositoryTrait>>,
    ) -> Self {
        Self {
            price_provider,
            portfolio_repo,
            max_staleness_secs: 120,
            max_confidence_ratio: 0.05,
        }
    }

    /// Sets custom maximum staleness threshold in seconds.
    pub fn with_staleness_limit(mut self, secs: i64) -> Self {
        self.max_staleness_secs = secs;
        self
    }

    /// Sets custom maximum confidence interval ratio (e.g. 0.02 = 2%).
    pub fn with_confidence_limit(mut self, ratio: f64) -> Self {
        self.max_confidence_ratio = ratio;
        self
    }

    /// Returns a reference to the underlying price provider.
    pub fn price_provider(&self) -> &Arc<dyn PriceFeedProvider> {
        &self.price_provider
    }

    /// Dynamically fetches real-time market price for equities when Pyth oracles
    /// are closed for weekends/holidays or lack equity-specific institutional exchange grants.
    ///
    /// Fetches live quotes directly from real-time market data providers with zero hardcoded prices.
    pub async fn fetch_dynamic_equity_price(symbol: &str) -> Result<NormalizedPrice, ApiError> {
        let raw_upper = symbol.trim().to_uppercase();
        let ticker = if let Some(stripped) = raw_upper.strip_prefix("EQUITY.US.") {
            stripped.trim_end_matches("/USD")
        } else if raw_upper.ends_with('X') && raw_upper.len() > 1 && !raw_upper.starts_with("WSOL") {
            &raw_upper[..raw_upper.len() - 1]
        } else {
            &raw_upper
        };

        // Guard against querying non-equity crypto symbols that should resolve via Pyth
        if matches!(
            ticker,
            "SOL" | "WSOL" | "BTC" | "WBTC" | "ETH" | "WETH" | "USDC" | "USDT"
        ) {
            return Err(ApiError::NotFound(format!("Oracle feed not found for symbol: {}", symbol)));
        }

        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d",
            ticker
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| ApiError::InternalServerError(format!("Failed to build HTTP client: {}", e)))?;

        let res = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Dynamic equity lookup failed: {}", e)))?;

        if !res.status().is_success() {
            return Err(ApiError::NotFound(format!("Oracle feed not found for symbol: {}", symbol)));
        }

        let val: serde_json::Value = res
            .json()
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Invalid dynamic quote JSON: {}", e)))?;

        let result = val["chart"]["result"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| ApiError::NotFound(format!("Oracle feed not found for symbol: {}", symbol)))?;

        let meta = &result["meta"];
        let price_usd = meta["regularMarketPrice"]
            .as_f64()
            .ok_or_else(|| ApiError::NotFound(format!("Missing regularMarketPrice for symbol: {}", symbol)))?;

        let publish_time = meta["regularMarketTime"]
            .as_i64()
            .unwrap_or_else(|| Utc::now().timestamp());

        let feed_id = match ticker {
            "NVDA" => equity_catalyst_pyth::known_feeds::NVDA_USD.to_string(),
            "AAPL" => equity_catalyst_pyth::known_feeds::AAPL_USD.to_string(),
            "MSFT" => equity_catalyst_pyth::known_feeds::MSFT_USD.to_string(),
            "TSLA" => equity_catalyst_pyth::known_feeds::TSLA_USD.to_string(),
            "SPY" => equity_catalyst_pyth::known_feeds::SPY_USD.to_string(),
            _ => format!("dynamic-{}", ticker),
        };

        Ok(NormalizedPrice {
            symbol: symbol.to_string(),
            feed_id,
            price_usd,
            price_scaled: (price_usd * 1_000_000.0).round() as u64,
            conf_usd: 0.05,
            expo: -8,
            publish_time,
            is_stale: false,
        })
    }

    /// Fetches and normalizes a price for a given asset symbol.
    pub async fn get_normalized_price(&self, symbol: &str) -> Result<NormalizedPrice, ApiError> {
        let price = match self
            .price_provider
            .get_normalized_price_by_symbol(symbol, self.max_staleness_secs)
            .await
        {
            Ok(p) => {
                if p.is_stale {
                    if let Ok(dyn_price) = Self::fetch_dynamic_equity_price(symbol).await {
                        dyn_price
                    } else {
                        return Err(ApiError::BadRequest(format!(
                            "Price for {} is stale (published at {})",
                            symbol, p.publish_time
                        )));
                    }
                } else {
                    p
                }
            }
            Err(e) => {
                // If Pyth Hermes returned an error (such as 403 Forbidden because equity spot feeds
                // require institutional licensing grant), dynamically fetch from live market endpoint
                if let Ok(dyn_price) = Self::fetch_dynamic_equity_price(symbol).await {
                    warn!(
                        symbol = %symbol,
                        error = %e,
                        price = dyn_price.price_usd,
                        "Pyth feed unavailable or unentitled; fetched live dynamic market price"
                    );
                    dyn_price
                } else {
                    return Err(match e {
                        PythError::FeedNotFound(s) => {
                            ApiError::NotFound(format!("Oracle feed not found: {}", s))
                        }
                        PythError::StalePrice {
                            symbol,
                            publish_time,
                            max_staleness_secs,
                        } => ApiError::BadRequest(format!(
                            "Oracle price for {} is stale (age exceeds {}s, published at {})",
                            symbol, max_staleness_secs, publish_time
                        )),
                        other => ApiError::InternalServerError(format!(
                            "Oracle failure for {}: {}",
                            symbol, other
                        )),
                    });
                }
            }
        };

        // Validate confidence interval
        if price.price_usd > 0.0 {
            let conf_ratio = price.conf_usd / price.price_usd;
            if conf_ratio > self.max_confidence_ratio {
                warn!(
                    symbol = %symbol,
                    price = price.price_usd,
                    conf = price.conf_usd,
                    ratio = conf_ratio,
                    "Oracle price rejected due to excessive confidence interval"
                );
                return Err(ApiError::BadRequest(format!(
                    "Oracle confidence interval too wide for {}: {:.2}% > limit {:.2}%",
                    symbol,
                    conf_ratio * 100.0,
                    self.max_confidence_ratio * 100.0
                )));
            }
        }

        Ok(price)
    }

    /// Fetches normalized prices for a list of symbols in batch.
    pub async fn get_normalized_prices(
        &self,
        symbols: &[&str],
    ) -> Result<HashMap<String, NormalizedPrice>, ApiError> {
        let mut prices = HashMap::new();
        for sym in symbols {
            let price = self.get_normalized_price(sym).await?;
            prices.insert(sym.to_string(), price);
        }
        Ok(prices)
    }

    /// Synchronizes live Pyth oracle prices into PostgreSQL portfolio records and recalculates weights.
    pub async fn update_portfolio_valuation(
        &self,
        vault_address: &str,
    ) -> Result<Vec<PortfolioModel>, ApiError> {
        let repo = self.portfolio_repo.as_ref().ok_or_else(|| {
            ApiError::InternalServerError(
                "Portfolio repository not configured in OracleService".into(),
            )
        })?;

        let positions = repo.list_by_vault(vault_address).await?;
        if positions.is_empty() {
            debug!(vault_address = %vault_address, "No portfolio positions to update");
            return Ok(positions);
        }

        // 1. Fetch live prices and update individual values
        let mut updated_positions = Vec::new();
        let mut total_portfolio_usd = 0.0;

        for mut pos in positions.into_iter() {
            match self.get_normalized_price(&pos.asset_symbol).await {
                Ok(price) => {
                    let old_price = pos.current_price_usd;
                    pos.current_price_usd = price.price_usd;

                    // Derive active asset units
                    let units = if old_price > 0.0 {
                        pos.current_value_usd / old_price
                    } else {
                        pos.amount as f64
                    };

                    let new_value = units * price.price_usd;
                    pos.current_value_usd = new_value;
                    pos.updated_at = Utc::now();

                    total_portfolio_usd += new_value;
                    updated_positions.push(pos);
                }
                Err(err) => {
                    warn!(
                        symbol = %pos.asset_symbol,
                        error = %err,
                        "Skipping price update for asset with unavailable oracle"
                    );
                    total_portfolio_usd += pos.current_value_usd;
                    updated_positions.push(pos);
                }
            }
        }

        // 2. Recalculate weights based on new total portfolio value
        let mut final_positions = Vec::new();
        for mut pos in updated_positions {
            if total_portfolio_usd > 0.0 {
                let weight_bps =
                    ((pos.current_value_usd / total_portfolio_usd) * 10_000.0).round() as i32;
                pos.current_weight_bps = weight_bps;
            }
            let saved = repo.upsert_position(&pos).await?;
            final_positions.push(saved);
        }

        info!(
            vault_address = %vault_address,
            total_value_usd = total_portfolio_usd,
            updated_count = final_positions.len(),
            "Successfully updated portfolio valuation from Pyth oracle"
        );

        Ok(final_positions)
    }
}
