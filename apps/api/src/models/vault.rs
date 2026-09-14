//! Vault domain model mapping to the `vaults` PostgreSQL table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultModel {
    pub vault_address: String,
    pub authority: String,
    pub name: String,
    pub symbol: String,
    pub deposit_mint: String,
    pub vault_token_account: String,
    pub total_shares: u64,
    pub total_deposits: u64,
    pub is_paused: bool,
    pub bump: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Row> for VaultModel {
    fn from(row: &Row) -> Self {
        let total_shares_i64: i64 = row.get("total_shares");
        let total_deposits_i64: i64 = row.get("total_deposits");

        Self {
            vault_address: row.get("vault_address"),
            authority: row.get("authority"),
            name: row.get("name"),
            symbol: row.get("symbol"),
            deposit_mint: row.get("deposit_mint"),
            vault_token_account: row.get("vault_token_account"),
            total_shares: total_shares_i64.max(0) as u64,
            total_deposits: total_deposits_i64.max(0) as u64,
            is_paused: row.get("is_paused"),
            bump: row.get("bump"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
