//! Portfolio position domain model mapping to the `portfolios` PostgreSQL table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioModel {
    pub portfolio_id: Uuid,
    pub vault_address: String,
    pub asset_symbol: String,
    pub asset_mint: String,
    pub amount: u64,
    pub entry_price_usd: f64,
    pub current_price_usd: f64,
    pub current_value_usd: f64,
    pub target_weight_bps: i32,
    pub current_weight_bps: i32,
    pub last_rebalanced_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Row> for PortfolioModel {
    fn from(row: &Row) -> Self {
        let amount_i64: i64 = row.get("amount");

        Self {
            portfolio_id: row.get("portfolio_id"),
            vault_address: row.get("vault_address"),
            asset_symbol: row.get("asset_symbol"),
            asset_mint: row.get("asset_mint"),
            amount: amount_i64.max(0) as u64,
            entry_price_usd: row.get("entry_price_usd"),
            current_price_usd: row.get("current_price_usd"),
            current_value_usd: row.get("current_value_usd"),
            target_weight_bps: row.get("target_weight_bps"),
            current_weight_bps: row.get("current_weight_bps"),
            last_rebalanced_at: row.get("last_rebalanced_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
