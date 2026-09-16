//! Data access repository for Event entities.

use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::{error::ApiError, models::EventModel};

#[derive(Clone, Debug)]
pub struct EventRepository {
    pool: Pool,
}

impl EventRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, event: &EventModel) -> Result<EventModel, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let row = client
            .query_one(
                r#"
                INSERT INTO events (
                    event_id, vault_address, event_type, source,
                    sentiment_score, payload, status, detected_at, processed_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING *
                "#,
                &[
                    &event.event_id,
                    &event.vault_address,
                    &event.event_type,
                    &event.source,
                    &event.sentiment_score,
                    &event.payload,
                    &event.status,
                    &event.detected_at,
                    &event.processed_at,
                ],
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to insert event: {}", e)))?;

        Ok(EventModel::from(&row))
    }

    pub async fn find_by_id(&self, event_id: Uuid) -> Result<Option<EventModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let row_opt = client
            .query_opt("SELECT * FROM events WHERE event_id = $1", &[&event_id])
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to query event: {}", e)))?;

        Ok(row_opt.map(|r| EventModel::from(&r)))
    }

    pub async fn list_by_vault(&self, vault_address: &str) -> Result<Vec<EventModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query(
                "SELECT * FROM events WHERE vault_address = $1 ORDER BY detected_at DESC",
                &[&vault_address],
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to list events: {}", e)))?;

        Ok(rows.iter().map(EventModel::from).collect())
    }

    pub async fn find_pending(&self) -> Result<Vec<EventModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query(
                "SELECT * FROM events WHERE status = 'PENDING' ORDER BY detected_at ASC",
                &[],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to list pending events: {}", e))
            })?;

        Ok(rows.iter().map(EventModel::from).collect())
    }

    pub async fn update_status(
        &self,
        event_id: Uuid,
        status: &str,
        processed_at: Option<DateTime<Utc>>,
    ) -> Result<(), ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        client
            .execute(
                r#"
                UPDATE events
                SET status = $2, processed_at = $3
                WHERE event_id = $1
                "#,
                &[&event_id, &status, &processed_at],
            )
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to update event status: {}", e))
            })?;

        Ok(())
    }

    pub async fn list_all(&self) -> Result<Vec<EventModel>, ApiError> {
        let client = self.pool.get().await.map_err(|e| {
            ApiError::InternalServerError(format!("Database connection failed: {}", e))
        })?;

        let rows = client
            .query("SELECT * FROM events ORDER BY detected_at DESC", &[])
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to list events: {}", e)))?;

        Ok(rows.iter().map(EventModel::from).collect())
    }
}
