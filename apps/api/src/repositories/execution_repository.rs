//! Data access repository for Execution entities.

use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::{error::ApiError, models::ExecutionModel};

#[derive(Clone, Debug)]
pub struct ExecutionRepository {
    pool: Pool,
}

impl ExecutionRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, execution: &ExecutionModel) -> Result<ExecutionModel, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let amount_out_actual_i64 = execution.amount_out_actual.map(|v| v as i64);

        let row = client
            .query_one(
                r#"
                INSERT INTO executions (
                    execution_id, vault_address, event_id, action, input_mint,
                    output_mint, amount_in, amount_out_expected, amount_out_actual,
                    slippage_bps, tx_signature, status, error_message, executed_at, confirmed_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                RETURNING *
                "#,
                &[
                    &execution.execution_id,
                    &execution.vault_address,
                    &execution.event_id,
                    &execution.action,
                    &execution.input_mint,
                    &execution.output_mint,
                    &(execution.amount_in as i64),
                    &(execution.amount_out_expected as i64),
                    &amount_out_actual_i64,
                    &execution.slippage_bps,
                    &execution.tx_signature,
                    &execution.status,
                    &execution.error_message,
                    &execution.executed_at,
                    &execution.confirmed_at,
                ],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to insert execution: {}", e))
            })?;

        Ok(ExecutionModel::from(&row))
    }

    pub async fn find_by_id(&self, execution_id: Uuid) -> Result<Option<ExecutionModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let row_opt = client
            .query_opt(
                "SELECT * FROM executions WHERE execution_id = $1",
                &[&execution_id],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to query execution: {}", e))
            })?;

        Ok(row_opt.map(|r| ExecutionModel::from(&r)))
    }

    pub async fn list_by_vault(
        &self,
        vault_address: &str,
    ) -> Result<Vec<ExecutionModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query(
                "SELECT * FROM executions WHERE vault_address = $1 ORDER BY executed_at DESC",
                &[&vault_address],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to list executions: {}", e))
            })?;

        Ok(rows.iter().map(ExecutionModel::from).collect())
    }

    pub async fn update_status(
        &self,
        execution_id: Uuid,
        status: &str,
        tx_signature: Option<&str>,
        amount_out_actual: Option<u64>,
        error_message: Option<&str>,
        confirmed_at: Option<DateTime<Utc>>,
    ) -> Result<(), ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let actual_out_i64 = amount_out_actual.map(|v| v as i64);

        client
            .execute(
                r#"
                UPDATE executions
                SET status = $2, tx_signature = COALESCE($3, tx_signature),
                    amount_out_actual = COALESCE($4, amount_out_actual),
                    error_message = $5, confirmed_at = $6
                WHERE execution_id = $1
                "#,
                &[
                    &execution_id,
                    &status,
                    &tx_signature,
                    &actual_out_i64,
                    &error_message,
                    &confirmed_at,
                ],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to update execution: {}", e))
            })?;

        Ok(())
    }

    pub async fn list_all(&self) -> Result<Vec<ExecutionModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query("SELECT * FROM executions ORDER BY executed_at DESC", &[])
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to list executions: {}", e))
            })?;

        Ok(rows.iter().map(ExecutionModel::from).collect())
    }
}
