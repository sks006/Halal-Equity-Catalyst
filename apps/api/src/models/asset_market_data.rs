//! Domain models mapping to the `asset_market_data` PostgreSQL table and composite views.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

/// Database row representation of `asset_market_data`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetMarketDataModel {
    pub mapping_id: Uuid,
    pub asset_id: String,
    pub pyth_feed_id: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Row> for AssetMarketDataModel {
    fn from(row: &Row) -> Self {
        Self {
            mapping_id: row.get("mapping_id"),
            asset_id: row.get("asset_id"),
            pyth_feed_id: row.get("pyth_feed_id"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// Composite projection joining canonical asset metadata with its Pyth feed mapping.
/// Used by market data consumers and subscription managers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetMarketDataMapping {
    pub mapping_id: Option<Uuid>,
    pub asset_id: String,
    pub symbol: String,
    pub mint_address: String,
    pub pyth_feed_id: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Row> for AssetMarketDataMapping {
    fn from(row: &Row) -> Self {
        let mapping_id: Option<Uuid> = row.try_get("mapping_id").ok();
        Self {
            mapping_id,
            asset_id: row.get("asset_id"),
            symbol: row.get("symbol"),
            mint_address: row.get("mint_address"),
            pyth_feed_id: row.get("pyth_feed_id"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// Request payload for creating a new asset <-> Pyth feed mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMarketDataMappingRequest {
    pub asset_id: String,
    pub pyth_feed_id: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}
