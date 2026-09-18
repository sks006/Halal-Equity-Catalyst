//! Data access repository for Dead-Letter records.

use chrono::Utc;
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::{error::ApiError, models::DeadLetterModel};

#[derive(Clone, Debug)]
pub struct DeadLetterRepository {
    pool: Pool,
}

impl DeadLetterRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn record_failure(
        &self,
        event_id: Option<Uuid>,
        failure_reason: &str,
        payload_reference: &serde_json::Value,
        retry_count: i32,
    ) -> Result<DeadLetterModel, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let dead_letter_id = Uuid::new_v4();
        let now = Utc::now();

        let row = client
            .query_one(
                r#"
                INSERT INTO dead_letters (
                    dead_letter_id, event_id, failure_reason, payload_reference, retry_count, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING *
                "#,
                &[
                    &dead_letter_id,
                    &event_id,
                    &failure_reason,
                    payload_reference,
                    &retry_count,
                    &now,
                ],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to insert dead letter: {}", e))
            })?;

        Ok(DeadLetterModel::from(&row))
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<DeadLetterModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query(
                "SELECT * FROM dead_letters ORDER BY created_at DESC LIMIT $1",
                &[&limit],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to list dead letters: {}", e))
            })?;

        Ok(rows.iter().map(DeadLetterModel::from).collect())
    }

    pub async fn find_by_event_id(&self, event_id: Uuid) -> Result<Vec<DeadLetterModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query(
                "SELECT * FROM dead_letters WHERE event_id = $1 ORDER BY created_at DESC",
                &[&event_id],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to query dead letters: {}", e))
            })?;

        Ok(rows.iter().map(DeadLetterModel::from).collect())
    }
}
