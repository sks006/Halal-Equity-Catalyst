//! Model for dead-letter records capturing failed, rejected, or unprocessable events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeadLetterModel {
    pub dead_letter_id: Uuid,
    pub event_id: Option<Uuid>,
    pub failure_reason: String,
    pub payload_reference: serde_json::Value,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
}

impl From<&Row> for DeadLetterModel {
    fn from(row: &Row) -> Self {
        Self {
            dead_letter_id: row.get("dead_letter_id"),
            event_id: row.get("event_id"),
            failure_reason: row.get("failure_reason"),
            payload_reference: row.get("payload_reference"),
            retry_count: row.get("retry_count"),
            created_at: row.get("created_at"),
        }
    }
}
