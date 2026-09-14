//! Data access repository for Vault entities.

use deadpool_postgres::Pool;

use crate::{error::ApiError, models::VaultModel};

#[derive(Clone, Debug)]
pub struct VaultRepository {
    pool: Pool,
}

impl VaultRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, vault: &VaultModel) -> Result<VaultModel, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let row = client
            .query_one(
                r#"
                INSERT INTO vaults (
                    vault_address, authority, name, symbol, deposit_mint,
                    vault_token_account, total_shares, total_deposits,
                    is_paused, bump, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                RETURNING *
                "#,
                &[
                    &vault.vault_address,
                    &vault.authority,
                    &vault.name,
                    &vault.symbol,
                    &vault.deposit_mint,
                    &vault.vault_token_account,
                    &(vault.total_shares as i64),
                    &(vault.total_deposits as i64),
                    &vault.is_paused,
                    &vault.bump,
                    &vault.created_at,
                    &vault.updated_at,
                ],
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to insert vault: {}", e)))?;

        Ok(VaultModel::from(&row))
    }

    pub async fn find_by_address(&self, address: &str) -> Result<Option<VaultModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let row_opt = client
            .query_opt(
                "SELECT * FROM vaults WHERE vault_address = $1",
                &[&address],
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to query vault: {}", e)))?;

        Ok(row_opt.map(|r| VaultModel::from(&r)))
    }

    pub async fn list_all(&self) -> Result<Vec<VaultModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query("SELECT * FROM vaults ORDER BY created_at DESC", &[])
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to list vaults: {}", e)))?;

        Ok(rows.iter().map(VaultModel::from).collect())
    }

    pub async fn update_totals(
        &self,
        address: &str,
        shares: u64,
        deposits: u64,
    ) -> Result<(), ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        client
            .execute(
                r#"
                UPDATE vaults
                SET total_shares = $2, total_deposits = $3, updated_at = NOW()
                WHERE vault_address = $1
                "#,
                &[&address, &(shares as i64), &(deposits as i64)],
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to update vault totals: {}", e)))?;

        Ok(())
    }

    pub async fn set_paused(&self, address: &str, paused: bool) -> Result<(), ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        client
            .execute(
                r#"
                UPDATE vaults
                SET is_paused = $2, updated_at = NOW()
                WHERE vault_address = $1
                "#,
                &[&address, &paused],
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to update pause state: {}", e)))?;

        Ok(())
    }
}
