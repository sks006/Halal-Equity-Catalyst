//! DBC Pool domain model mapping to the `dbc_pools` PostgreSQL table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbcPoolModel {
    pub pool_address: String,
    pub config_address: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub token_name: String,
    pub token_symbol: String,
    pub tx_signature: String,
    pub creator: String,
    pub initial_price_usd: f64,
    pub current_price_usd: f64,
    pub curve_progress_pct: f64,
    pub is_migrated: bool,
    pub creation_timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDbcPoolRequest {
    pub pool_address: String,
    pub config_address: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub token_name: String,
    pub token_symbol: String,
    pub tx_signature: String,
    pub creator: String,
    #[serde(default)]
    pub initial_price_usd: f64,
    #[serde(default)]
    pub current_price_usd: f64,
    #[serde(default)]
    pub curve_progress_pct: f64,
    #[serde(default)]
    pub is_migrated: bool,
    pub creation_timestamp: Option<DateTime<Utc>>,
}

impl From<&Row> for DbcPoolModel {
    fn from(row: &Row) -> Self {
        Self {
            pool_address: row.get("pool_address"),
            config_address: row.get("config_address"),
            base_mint: row.get("base_mint"),
            quote_mint: row.get("quote_mint"),
            token_name: row.get("token_name"),
            token_symbol: row.get("token_symbol"),
            tx_signature: row.get("tx_signature"),
            creator: row.get("creator"),
            initial_price_usd: row.get("initial_price_usd"),
            current_price_usd: row.get("current_price_usd"),
            curve_progress_pct: row.get("curve_progress_pct"),
            is_migrated: row.get("is_migrated"),
            creation_timestamp: row.get("creation_timestamp"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
