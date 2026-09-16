//! Data access repository for Portfolio position entities.

use deadpool_postgres::Pool;

use crate::{error::ApiError, models::PortfolioModel};

#[derive(Clone, Debug)]
pub struct PortfolioRepository {
    pool: Pool,
}

impl PortfolioRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn upsert_position(&self, pos: &PortfolioModel) -> Result<PortfolioModel, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let row = client
            .query_one(
                r#"
                INSERT INTO portfolios (
                    portfolio_id, vault_address, asset_symbol, asset_mint, amount,
                    entry_price_usd, current_price_usd, current_value_usd,
                    target_weight_bps, current_weight_bps, last_rebalanced_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (vault_address, asset_symbol) DO UPDATE SET
                    asset_mint = EXCLUDED.asset_mint,
                    amount = EXCLUDED.amount,
                    entry_price_usd = EXCLUDED.entry_price_usd,
                    current_price_usd = EXCLUDED.current_price_usd,
                    current_value_usd = EXCLUDED.current_value_usd,
                    target_weight_bps = EXCLUDED.target_weight_bps,
                    current_weight_bps = EXCLUDED.current_weight_bps,
                    last_rebalanced_at = EXCLUDED.last_rebalanced_at,
                    updated_at = NOW()
                RETURNING *
                "#,
                &[
                    &pos.portfolio_id,
                    &pos.vault_address,
                    &pos.asset_symbol,
                    &pos.asset_mint,
                    &(pos.amount as i64),
                    &pos.entry_price_usd,
                    &pos.current_price_usd,
                    &pos.current_value_usd,
                    &pos.target_weight_bps,
                    &pos.current_weight_bps,
                    &pos.last_rebalanced_at,
                    &pos.updated_at,
                ],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to upsert portfolio: {}", e))
            })?;

        Ok(PortfolioModel::from(&row))
    }

    pub async fn list_by_vault(
        &self,
        vault_address: &str,
    ) -> Result<Vec<PortfolioModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query(
                "SELECT * FROM portfolios WHERE vault_address = $1 ORDER BY current_value_usd DESC",
                &[&vault_address],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to list portfolio: {}", e))
            })?;

        Ok(rows.iter().map(PortfolioModel::from).collect())
    }
}
