//! ExecutionRequest and trade order data structures produced by the Decision Engine.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOrder {
    pub symbol: String,
    pub is_buy: bool,
    pub usd_value: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub decision_id: Uuid,
    pub vault_address: String,
    pub event_id: Option<Uuid>,
    pub action: String,
    pub trades: Vec<TradeOrder>,
    pub approved: bool,
    pub rationale: String,
    pub timestamp: DateTime<Utc>,
}
