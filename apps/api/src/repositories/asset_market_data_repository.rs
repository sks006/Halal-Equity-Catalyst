//! Data access repository for Asset <-> Pyth Feed Market Data Mappings.
//!
//! Enforces explicit database-backed feed resolution, single-active-mapping constraints,
//! and fail-closed validation requiring prior Shariah compliance approval.

use chrono::Utc;
use deadpool_postgres::Pool;
use equity_catalyst_shared::AssetApprovalStatus;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::{
    error::ApiError,
    models::{
        asset_market_data::{
            subscription_set_from_mappings, AssetMarketDataMapping, AssetMarketDataModel,
        },
        canonical_asset::CanonicalAssetModel,
    },
    repositories::CanonicalAssetRepository,
};
use equity_catalyst_pyth::SubscriptionSet;

#[derive(Clone, Debug)]
enum Backend {
    Postgres(Pool),
    InMemory(Arc<RwLock<HashMap<String, AssetMarketDataModel>>>),
}

/// Repository for persisting and querying explicit asset-to-Pyth price feed mappings.
#[derive(Clone, Debug)]
pub struct AssetMarketDataRepository {
    backend: Backend,
    asset_repo: CanonicalAssetRepository,
}

impl AssetMarketDataRepository {
    /// Creates a repository backed by PostgreSQL.
    pub fn new(pool: Pool, asset_repo: CanonicalAssetRepository) -> Self {
        Self {
            backend: Backend::Postgres(pool),
            asset_repo,
        }
    }

    /// Creates an in-memory repository for deterministic testing and offline execution.
    pub fn new_in_memory(asset_repo: CanonicalAssetRepository) -> Self {
        Self {
            backend: Backend::InMemory(Arc::new(RwLock::new(HashMap::new()))),
            asset_repo,
        }
    }

    /// Normalizes and validates a Pyth Hermes hex feed ID.
    fn normalize_feed_id(feed_id: &str) -> Result<String, ApiError> {
        let clean = feed_id.trim_start_matches("0x").trim().to_lowercase();
        if clean.is_empty() {
            return Err(ApiError::BadRequest(
                "Pyth feed ID cannot be empty".to_string(),
            ));
        }
        Ok(clean)
    }

    /// Creates an explicit mapping between an approved canonical asset and a Pyth price feed.
    ///
    /// Validations:
    /// 1. Asset must exist in the canonical asset registry.
    /// 2. Asset must have attained Shariah compliance approval.
    /// 3. Pyth feed ID must be non-empty.
    /// 4. Asset cannot already have an active mapping (one active mapping per asset).
    /// 5. Pyth feed ID cannot already be mapped to another active asset.
    pub async fn create_mapping(
        &self,
        asset_id: &str,
        pyth_feed_id: &str,
    ) -> Result<AssetMarketDataMapping, ApiError> {
        let clean_feed_id = Self::normalize_feed_id(pyth_feed_id)?;

        // 1. Verify asset exists
        let asset = self
            .asset_repo
            .get_asset_by_id(asset_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Asset '{}' does not exist", asset_id)))?;

        // 2. Verify asset is approved (must be ShariahApproved or Active)
        if !asset.approval_status.is_shariah_approved() {
            return Err(ApiError::BadRequest(format!(
                "Cannot map price feed to unapproved asset '{}': current status is '{}'",
                asset_id, asset.approval_status
            )));
        }

        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let row = client
                    .query_one(
                        r#"
                        INSERT INTO asset_market_data (asset_id, pyth_feed_id, is_active, created_at, updated_at)
                        VALUES ($1, $2, TRUE, NOW(), NOW())
                        RETURNING *
                        "#,
                        &[&asset_id, &clean_feed_id],
                    )
                    .await
                    .map_err(|e| {
                        let err_msg = e.to_string();
                        if err_msg.contains("idx_asset_market_data_active_asset")
                            || err_msg.contains("duplicate key value")
                        {
                            ApiError::BadRequest(format!(
                                "Active mapping already exists for asset '{}'",
                                asset_id
                            ))
                        } else if err_msg.contains("idx_asset_market_data_active_feed") {
                            ApiError::BadRequest(format!(
                                "Pyth feed '{}' is already mapped to an active asset",
                                clean_feed_id
                            ))
                        } else {
                            ApiError::InternalServerError(format!(
                                "Failed to insert asset market data mapping: {}",
                                e
                            ))
                        }
                    })?;

                let model = AssetMarketDataModel::from(&row);
                Ok(AssetMarketDataMapping {
                    mapping_id: Some(model.mapping_id),
                    asset_id: asset.asset_id,
                    symbol: asset.symbol,
                    mint_address: asset.mint_address,
                    pyth_feed_id: model.pyth_feed_id,
                    is_active: model.is_active,
                    created_at: model.created_at,
                    updated_at: model.updated_at,
                })
            }
            Backend::InMemory(store) => {
                let mut guard = store.write().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;

                // Check active mapping per asset
                if guard
                    .values()
                    .any(|m| m.asset_id == asset_id && m.is_active)
                {
                    return Err(ApiError::BadRequest(format!(
                        "Active mapping already exists for asset '{}'",
                        asset_id
                    )));
                }

                // Check active mapping per feed
                if guard
                    .values()
                    .any(|m| m.pyth_feed_id == clean_feed_id && m.is_active)
                {
                    return Err(ApiError::BadRequest(format!(
                        "Pyth feed '{}' is already mapped to an active asset",
                        clean_feed_id
                    )));
                }

                let now = Utc::now();
                let mapping_id = Uuid::new_v4();
                let model = AssetMarketDataModel {
                    mapping_id,
                    asset_id: asset_id.to_string(),
                    pyth_feed_id: clean_feed_id.clone(),
                    is_active: true,
                    created_at: now,
                    updated_at: now,
                };

                guard.insert(asset_id.to_string(), model);

                Ok(AssetMarketDataMapping {
                    mapping_id: Some(mapping_id),
                    asset_id: asset.asset_id,
                    symbol: asset.symbol,
                    mint_address: asset.mint_address,
                    pyth_feed_id: clean_feed_id,
                    is_active: true,
                    created_at: now,
                    updated_at: now,
                })
            }
        }
    }

    /// Retrieves the market data mapping for a specific asset.
    pub async fn get_mapping_for_asset(
        &self,
        asset_id: &str,
    ) -> Result<Option<AssetMarketDataMapping>, ApiError> {
        let asset_opt = self.asset_repo.get_asset_by_id(asset_id).await?;
        let asset = match asset_opt {
            Some(a) => a,
            None => return Ok(None),
        };

        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let row_opt = client
                    .query_opt(
                        r#"
                        SELECT mapping_id, asset_id, pyth_feed_id, is_active, created_at, updated_at
                        FROM asset_market_data
                        WHERE asset_id = $1
                        ORDER BY updated_at DESC
                        LIMIT 1
                        "#,
                        &[&asset_id],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalServerError(format!(
                            "Failed to query market data mapping: {}",
                            e
                        ))
                    })?;

                Ok(row_opt.map(|r| {
                    let model = AssetMarketDataModel::from(&r);
                    AssetMarketDataMapping {
                        mapping_id: Some(model.mapping_id),
                        asset_id: asset.asset_id.clone(),
                        symbol: asset.symbol.clone(),
                        mint_address: asset.mint_address.clone(),
                        pyth_feed_id: model.pyth_feed_id,
                        is_active: model.is_active,
                        created_at: model.created_at,
                        updated_at: model.updated_at,
                    }
                }))
            }
            Backend::InMemory(store) => {
                let guard = store.read().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;

                Ok(guard.get(asset_id).map(|m| AssetMarketDataMapping {
                    mapping_id: Some(m.mapping_id),
                    asset_id: asset.asset_id.clone(),
                    symbol: asset.symbol.clone(),
                    mint_address: asset.mint_address.clone(),
                    pyth_feed_id: m.pyth_feed_id.clone(),
                    is_active: m.is_active,
                    created_at: m.created_at,
                    updated_at: m.updated_at,
                }))
            }
        }
    }

    /// Alias for `get_mapping_for_asset`.
    pub async fn get_pyth_mapping(
        &self,
        asset_id: &str,
    ) -> Result<Option<AssetMarketDataMapping>, ApiError> {
        self.get_mapping_for_asset(asset_id).await
    }

    /// Answers the core acceptance criterion:

    /// "For every currently active approved asset, which Pyth feed should provide its reference price?"
    ///
    /// Excludes:
    /// - Inactive mappings
    /// - Inactive assets
    /// - Non-approved assets
    pub async fn get_active_mappings(&self) -> Result<Vec<AssetMarketDataMapping>, ApiError> {
        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let rows = client
                    .query(
                        r#"
                        SELECT 
                            m.mapping_id,
                            m.asset_id,
                            a.symbol,
                            a.mint_address,
                            m.pyth_feed_id,
                            m.is_active,
                            m.created_at,
                            m.updated_at
                        FROM asset_market_data m
                        INNER JOIN canonical_assets a ON m.asset_id = a.asset_id
                        WHERE m.is_active = TRUE
                          AND a.is_active = TRUE
                          AND a.approval_status = 'ACTIVE'
                        ORDER BY a.symbol ASC
                        "#,
                        &[],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalServerError(format!(
                            "Failed to query active market data mappings: {}",
                            e
                        ))
                    })?;

                Ok(rows.iter().map(AssetMarketDataMapping::from).collect())
            }
            Backend::InMemory(store) => {
                let active_models: Vec<AssetMarketDataModel> = {
                    let guard = store.read().map_err(|e| {
                        ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                    })?;
                    guard.values().filter(|m| m.is_active).cloned().collect()
                };

                let mut active_list = Vec::new();
                for model in active_models {
                    if let Some(asset) = self.asset_repo.get_asset_by_id(&model.asset_id).await? {
                        // Strict dual-check: Asset must be ACTIVE and SHARIAH_APPROVED
                        if asset.is_active && asset.approval_status == AssetApprovalStatus::Active {
                            active_list.push(AssetMarketDataMapping {
                                mapping_id: Some(model.mapping_id),
                                asset_id: asset.asset_id,
                                symbol: asset.symbol,
                                mint_address: asset.mint_address,
                                pyth_feed_id: model.pyth_feed_id.clone(),
                                is_active: true,
                                created_at: model.created_at,
                                updated_at: model.updated_at,
                            });
                        }
                    }
                }

                active_list.sort_by(|a, b| a.symbol.cmp(&b.symbol));
                Ok(active_list)
            }
        }
    }

    /// Alias for `get_active_mappings`.
    pub async fn list_active_pyth_mappings(&self) -> Result<Vec<AssetMarketDataMapping>, ApiError> {
        self.get_active_mappings().await
    }

    /// Delegates to underlying `CanonicalAssetRepository` to list active approved assets.
    pub async fn list_active_approved_assets(&self) -> Result<Vec<CanonicalAssetModel>, ApiError> {
        self.asset_repo.list_active_approved_assets().await
    }

    /// Delegates to underlying `CanonicalAssetRepository` to get an asset by its ID.
    pub async fn get_asset(&self, asset_id: &str) -> Result<Option<CanonicalAssetModel>, ApiError> {
        self.asset_repo.get_asset(asset_id).await
    }

    /// Delegates to underlying `CanonicalAssetRepository` to get an asset by its mint.
    pub async fn get_by_mint(
        &self,
        mint_address: &str,
    ) -> Result<Option<CanonicalAssetModel>, ApiError> {
        self.asset_repo.get_by_mint(mint_address).await
    }

    /// Builds a deterministic `SubscriptionSet` directly from the database's active approved mappings.
    pub async fn build_subscription_set(&self) -> Result<SubscriptionSet, ApiError> {
        let mappings = self.list_active_pyth_mappings().await?;
        Ok(subscription_set_from_mappings(&mappings))
    }

    /// Deactivates a market data feed mapping for an asset.

    ///
    /// Immediately prevents the feed from being included in live subscriptions.
    pub async fn deactivate_mapping(
        &self,
        asset_id: &str,
    ) -> Result<AssetMarketDataMapping, ApiError> {
        let asset = self
            .asset_repo
            .get_asset_by_id(asset_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Asset '{}' not found", asset_id)))?;

        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let row = client
                    .query_one(
                        r#"
                        UPDATE asset_market_data
                        SET is_active = FALSE, updated_at = NOW()
                        WHERE asset_id = $1 AND is_active = TRUE
                        RETURNING *
                        "#,
                        &[&asset_id],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::NotFound(format!(
                            "No active mapping found to deactivate for asset '{}': {}",
                            asset_id, e
                        ))
                    })?;

                let model = AssetMarketDataModel::from(&row);
                Ok(AssetMarketDataMapping {
                    mapping_id: Some(model.mapping_id),
                    asset_id: asset.asset_id,
                    symbol: asset.symbol,
                    mint_address: asset.mint_address,
                    pyth_feed_id: model.pyth_feed_id,
                    is_active: false,
                    created_at: model.created_at,
                    updated_at: model.updated_at,
                })
            }
            Backend::InMemory(store) => {
                let mut guard = store.write().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;

                let mapping = guard.get_mut(asset_id).ok_or_else(|| {
                    ApiError::NotFound(format!("No mapping found for asset '{}'", asset_id))
                })?;

                mapping.is_active = false;
                mapping.updated_at = Utc::now();

                Ok(AssetMarketDataMapping {
                    mapping_id: Some(mapping.mapping_id),
                    asset_id: asset.asset_id,
                    symbol: asset.symbol,
                    mint_address: asset.mint_address,
                    pyth_feed_id: mapping.pyth_feed_id.clone(),
                    is_active: false,
                    created_at: mapping.created_at,
                    updated_at: mapping.updated_at,
                })
            }
        }
    }

    /// Updates the Pyth price feed ID for an approved asset.
    pub async fn update_feed_mapping(
        &self,
        asset_id: &str,
        new_feed_id: &str,
    ) -> Result<AssetMarketDataMapping, ApiError> {
        let clean_feed_id = Self::normalize_feed_id(new_feed_id)?;

        let asset = self
            .asset_repo
            .get_asset_by_id(asset_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Asset '{}' not found", asset_id)))?;

        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let row = client
                    .query_one(
                        r#"
                        UPDATE asset_market_data
                        SET pyth_feed_id = $2, updated_at = NOW()
                        WHERE asset_id = $1
                        RETURNING *
                        "#,
                        &[&asset_id, &clean_feed_id],
                    )
                    .await
                    .map_err(|e| {
                        let err_msg = e.to_string();
                        if err_msg.contains("idx_asset_market_data_active_feed") {
                            ApiError::BadRequest(format!(
                                "Pyth feed '{}' is already mapped to another active asset",
                                clean_feed_id
                            ))
                        } else {
                            ApiError::NotFound(format!(
                                "No mapping found for asset '{}' to update: {}",
                                asset_id, e
                            ))
                        }
                    })?;

                let model = AssetMarketDataModel::from(&row);
                Ok(AssetMarketDataMapping {
                    mapping_id: Some(model.mapping_id),
                    asset_id: asset.asset_id,
                    symbol: asset.symbol,
                    mint_address: asset.mint_address,
                    pyth_feed_id: model.pyth_feed_id,
                    is_active: model.is_active,
                    created_at: model.created_at,
                    updated_at: model.updated_at,
                })
            }
            Backend::InMemory(store) => {
                let mut guard = store.write().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;

                // Check if another active asset already uses clean_feed_id
                if guard.values().any(|m| {
                    m.asset_id != asset_id && m.pyth_feed_id == clean_feed_id && m.is_active
                }) {
                    return Err(ApiError::BadRequest(format!(
                        "Pyth feed '{}' is already mapped to another active asset",
                        clean_feed_id
                    )));
                }

                let mapping = guard.get_mut(asset_id).ok_or_else(|| {
                    ApiError::NotFound(format!("No mapping found for asset '{}'", asset_id))
                })?;

                mapping.pyth_feed_id = clean_feed_id.clone();
                mapping.updated_at = Utc::now();

                Ok(AssetMarketDataMapping {
                    mapping_id: Some(mapping.mapping_id),
                    asset_id: asset.asset_id,
                    symbol: asset.symbol,
                    mint_address: asset.mint_address,
                    pyth_feed_id: clean_feed_id,
                    is_active: mapping.is_active,
                    created_at: mapping.created_at,
                    updated_at: mapping.updated_at,
                })
            }
        }
    }
}
