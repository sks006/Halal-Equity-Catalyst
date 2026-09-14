//! Execution domain model mapping to the `executions` PostgreSQL table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionModel {
    pub execution_id: Uuid,
    pub vault_address: String,
    pub event_id: Option<Uuid>,
    pub action: String,
    pub input_mint: String,
    pub output_mint: String,
    pub amount_in: u64,
    pub amount_out_expected: u64,
    pub amount_out_actual: Option<u64>,
    pub slippage_bps: i32,
    pub tx_signature: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub executed_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

impl From<&Row> for ExecutionModel {
    fn from(row: &Row) -> Self {
        let amount_in_i64: i64 = row.get("amount_in");
        let amount_out_expected_i64: i64 = row.get("amount_out_expected");
        let amount_out_actual_i64: Option<i64> = row.get("amount_out_actual");

        Self {
            execution_id: row.get("execution_id"),
            vault_address: row.get("vault_address"),
            event_id: row.get("event_id"),
            action: row.get("action"),
            input_mint: row.get("input_mint"),
            output_mint: row.get("output_mint"),
            amount_in: amount_in_i64.max(0) as u64,
            amount_out_expected: amount_out_expected_i64.max(0) as u64,
            amount_out_actual: amount_out_actual_i64.map(|v| v.max(0) as u64),
            slippage_bps: row.get("slippage_bps"),
            tx_signature: row.get("tx_signature"),
            status: row.get("status"),
            error_message: row.get("error_message"),
            executed_at: row.get("executed_at"),
            confirmed_at: row.get("confirmed_at"),
        }
    }
}
