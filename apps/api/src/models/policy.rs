//! Policy domain model mapping to the `policies` PostgreSQL table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyModel {
    pub policy_address: String,
    pub vault_address: String,
    pub authority: String,
    pub min_cash_bps: i32,
    pub max_position_bps: i32,
    pub stop_loss_bps: i32,
    pub take_profit_bps: i32,
    pub rebalance_threshold_bps: i32,
    pub is_active: bool,
    pub bump: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Row> for PolicyModel {
    fn from(row: &Row) -> Self {
        Self {
            policy_address: row.get("policy_address"),
            vault_address: row.get("vault_address"),
            authority: row.get("authority"),
            min_cash_bps: row.get("min_cash_bps"),
            max_position_bps: row.get("max_position_bps"),
            stop_loss_bps: row.get("stop_loss_bps"),
            take_profit_bps: row.get("take_profit_bps"),
            rebalance_threshold_bps: row.get("rebalance_threshold_bps"),
            is_active: row.get("is_active"),
            bump: row.get("bump"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
