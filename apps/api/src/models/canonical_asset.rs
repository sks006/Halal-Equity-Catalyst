//! Canonical asset domain model mapping to the `canonical_assets` PostgreSQL table.

use chrono::{DateTime, Utc};
use equity_catalyst_shared::AssetApprovalStatus;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

fn default_decimals() -> i16 {
    6
}

/// Domain representation of a canonical tokenized equity asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalAssetModel {
    pub asset_id: String,
    pub symbol: String,
    pub mint_address: String,
    pub legal_issuer: String,
    pub custodian: String,
    pub underlying_asset_identifier: String,
    pub is_active: bool,
    pub approval_status: AssetApprovalStatus,
    pub decimals: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request payload for creating a new canonical asset entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssetRequest {
    pub asset_id: String,
    pub symbol: String,
    pub mint_address: String,
    pub legal_issuer: String,
    pub custodian: String,
    pub underlying_asset_identifier: String,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub approval_status: Option<AssetApprovalStatus>,
    #[serde(default = "default_decimals")]
    pub decimals: i16,
}

impl From<&Row> for CanonicalAssetModel {
    fn from(row: &Row) -> Self {
        let status_str: String = row.get("approval_status");
        let approval_status = status_str.parse().unwrap_or(AssetApprovalStatus::Pending);

        Self {
            asset_id: row.get("asset_id"),
            symbol: row.get("symbol"),
            mint_address: row.get("mint_address"),
            legal_issuer: row.get("legal_issuer"),
            custodian: row.get("custodian"),
            underlying_asset_identifier: row.get("underlying_asset_identifier"),
            is_active: row.get("is_active"),
            approval_status,
            decimals: row.get("decimals"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
