//! Event domain model mapping to the `events` PostgreSQL table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventModel {
    pub event_id: Uuid,
    pub vault_address: Option<String>,
    pub event_type: String,
    pub source: String,
    pub sentiment_score: Option<f64>,
    pub payload: serde_json::Value,
    pub status: String,
    pub detected_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

impl From<&Row> for EventModel {
    fn from(row: &Row) -> Self {
        Self {
            event_id: row.get("event_id"),
            vault_address: row.get("vault_address"),
            event_type: row.get("event_type"),
            source: row.get("source"),
            sentiment_score: row.get("sentiment_score"),
            payload: row.get("payload"),
            status: row.get("status"),
            detected_at: row.get("detected_at"),
            processed_at: row.get("processed_at"),
        }
    }
}
