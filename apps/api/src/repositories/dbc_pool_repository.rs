//! Data access repository for DBC Pool entities.

use deadpool_postgres::Pool;

use crate::{error::ApiError, models::dbc_pool::{CreateDbcPoolRequest, DbcPoolModel}};

#[derive(Clone, Debug)]
pub struct DbcPoolRepository {
    pool: Pool,
}

impl DbcPoolRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, req: &CreateDbcPoolRequest) -> Result<DbcPoolModel, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let row = client
            .query_one(
                r#"
                INSERT INTO dbc_pools (
                    pool_address, config_address, base_mint, quote_mint,
                    token_name, token_symbol, tx_signature, creator,
                    initial_price_usd, current_price_usd, curve_progress_pct,
                    is_migrated, creation_timestamp, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, COALESCE($13, NOW()), NOW(), NOW())
                ON CONFLICT (pool_address) DO UPDATE
                SET
                    config_address = EXCLUDED.config_address,
                    base_mint = EXCLUDED.base_mint,
                    quote_mint = EXCLUDED.quote_mint,
                    token_name = EXCLUDED.token_name,
                    token_symbol = EXCLUDED.token_symbol,
                    tx_signature = EXCLUDED.tx_signature,
                    creator = EXCLUDED.creator,
                    initial_price_usd = EXCLUDED.initial_price_usd,
                    current_price_usd = EXCLUDED.current_price_usd,
                    curve_progress_pct = EXCLUDED.curve_progress_pct,
                    is_migrated = EXCLUDED.is_migrated,
                    updated_at = NOW()
                RETURNING *
                "#,
                &[
                    &req.pool_address,
                    &req.config_address,
                    &req.base_mint,
                    &req.quote_mint,
                    &req.token_name,
                    &req.token_symbol,
                    &req.tx_signature,
                    &req.creator,
                    &req.initial_price_usd,
                    &req.current_price_usd,
                    &req.curve_progress_pct,
                    &req.is_migrated,
                    &req.creation_timestamp,
                ],
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to insert dbc pool: {}", e)))?;

        Ok(DbcPoolModel::from(&row))
    }

    pub async fn find_by_pool_address(&self, pool_address: &str) -> Result<Option<DbcPoolModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let row_opt = client
            .query_opt(
                "SELECT * FROM dbc_pools WHERE pool_address = $1",
                &[&pool_address],
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to query dbc pool: {}", e)))?;

        Ok(row_opt.map(|r| DbcPoolModel::from(&r)))
    }

    pub async fn list_all(&self) -> Result<Vec<DbcPoolModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query("SELECT * FROM dbc_pools ORDER BY creation_timestamp DESC", &[])
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to list dbc pools: {}", e)))?;

        Ok(rows.iter().map(DbcPoolModel::from).collect())
    }

    pub async fn update_price_and_progress(
        &self,
        pool_address: &str,
        current_price_usd: f64,
        curve_progress_pct: f64,
        is_migrated: bool,
    ) -> Result<(), ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        client
            .execute(
                r#"
                UPDATE dbc_pools
                SET current_price_usd = $2, curve_progress_pct = $3, is_migrated = $4, updated_at = NOW()
                WHERE pool_address = $1
                "#,
                &[&pool_address, &current_price_usd, &curve_progress_pct, &is_migrated],
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to update dbc pool telemetry: {}", e)))?;

        Ok(())
    }
}
