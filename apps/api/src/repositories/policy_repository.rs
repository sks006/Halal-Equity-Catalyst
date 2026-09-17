//! Data access repository for Policy entities.

use deadpool_postgres::Pool;

use crate::{error::ApiError, models::PolicyModel};

#[derive(Clone, Debug)]
pub struct PolicyRepository {
    pool: Pool,
}

impl PolicyRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, policy: &PolicyModel) -> Result<PolicyModel, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let row = client
            .query_one(
                r#"
                INSERT INTO policies (
                    policy_address, vault_address, authority, min_cash_bps,
                    max_position_bps, stop_loss_bps, take_profit_bps,
                    rebalance_threshold_bps, is_active, bump, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (vault_address) DO UPDATE SET
                    authority = EXCLUDED.authority,
                    min_cash_bps = EXCLUDED.min_cash_bps,
                    max_position_bps = EXCLUDED.max_position_bps,
                    stop_loss_bps = EXCLUDED.stop_loss_bps,
                    take_profit_bps = EXCLUDED.take_profit_bps,
                    rebalance_threshold_bps = EXCLUDED.rebalance_threshold_bps,
                    is_active = EXCLUDED.is_active,
                    updated_at = NOW()
                RETURNING *
                "#,
                &[
                    &policy.policy_address,
                    &policy.vault_address,
                    &policy.authority,
                    &policy.min_cash_bps,
                    &policy.max_position_bps,
                    &policy.stop_loss_bps,
                    &policy.take_profit_bps,
                    &policy.rebalance_threshold_bps,
                    &policy.is_active,
                    &policy.bump,
                    &policy.created_at,
                    &policy.updated_at,
                ],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to upsert policy: {}", e))
            })?;

        Ok(PolicyModel::from(&row))
    }

    pub async fn find_by_vault(
        &self,
        vault_address: &str,
    ) -> Result<Option<PolicyModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let row_opt = client
            .query_opt(
                "SELECT * FROM policies WHERE vault_address = $1",
                &[&vault_address],
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to query policy: {}", e)))?;

        Ok(row_opt.map(|r| PolicyModel::from(&r)))
    }

    pub async fn list_all(&self) -> Result<Vec<PolicyModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query("SELECT * FROM policies ORDER BY created_at DESC", &[])
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to list policies: {}", e))
            })?;

        Ok(rows.iter().map(PolicyModel::from).collect())
    }
}
