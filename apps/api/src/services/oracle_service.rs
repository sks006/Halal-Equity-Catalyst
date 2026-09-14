//! Oracle domain service managing Pyth price feeds and portfolio valuations.

use chrono::Utc;
use equity_catalyst_pyth::{NormalizedPrice, PythClient, PythError};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::{
    error::ApiError,
    models::PortfolioModel,
    repositories::PortfolioRepository,
};

/// Service responsible for fetching, validating, and normalizing external oracle price feeds.
#[derive(Clone)]
pub struct OracleService {
    pyth_client: Arc<PythClient>,
    portfolio_repo: Option<PortfolioRepository>,
    max_staleness_secs: i64,
    max_confidence_ratio: f64,
}

impl OracleService {
    /// Creates a new OracleService with standard safety thresholds.
    pub fn new(
        pyth_client: Arc<PythClient>,
        portfolio_repo: Option<PortfolioRepository>,
    ) -> Self {
        Self {
            pyth_client,
            portfolio_repo,
            max_staleness_secs: 120,    // 2 minutes max staleness
            max_confidence_ratio: 0.05, // Max 5% confidence width
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

    /// Returns a reference to the underlying Pyth client.
    pub fn pyth_client(&self) -> &PythClient {
        &self.pyth_client
    }

    /// Fetches and normalizes a price for a given asset symbol.
    pub async fn get_normalized_price(&self, symbol: &str) -> Result<NormalizedPrice, ApiError> {
        let price = self
            .pyth_client
            .get_normalized_price_by_symbol(symbol, self.max_staleness_secs)
            .await
            .map_err(|e| match e {
                PythError::FeedNotFound(s) => ApiError::NotFound(format!("Oracle feed not found: {}", s)),
                PythError::StalePrice { symbol, publish_time, max_staleness_secs } => {
                    ApiError::BadRequest(format!(
                        "Oracle price for {} is stale (age exceeds {}s, published at {})",
                        symbol, max_staleness_secs, publish_time
                    ))
                }
                other => ApiError::InternalServerError(format!("Oracle failure for {}: {}", symbol, other)),
            })?;

        // Validate staleness
        if price.is_stale {
            return Err(ApiError::BadRequest(format!(
                "Price for {} is stale (published at {})",
                symbol, price.publish_time
            )));
        }

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
                    symbol, conf_ratio * 100.0, self.max_confidence_ratio * 100.0
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
            ApiError::InternalServerError("Portfolio repository not configured in OracleService".into())
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
                let weight_bps = ((pos.current_value_usd / total_portfolio_usd) * 10_000.0).round() as i32;
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
