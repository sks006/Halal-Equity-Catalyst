//! Data access repository for Canonical Asset entities.
//!
//! Enforces canonical identity rooted in SPL token mint address,
//! non-symbol primary keys, unique mint constraints, and lifecycle state machines.

use chrono::Utc;
use deadpool_postgres::Pool;
use equity_catalyst_shared::{AssetApprovalStatus, ValidationError};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::{
    error::ApiError,
    models::canonical_asset::{CanonicalAssetModel, CreateAssetRequest},
};

#[derive(Clone, Debug)]
enum Backend {
    Postgres(Pool),
    InMemory(Arc<RwLock<HashMap<String, CanonicalAssetModel>>>),
}

/// Repository for persisting and managing canonical assets.
#[derive(Clone, Debug)]
pub struct CanonicalAssetRepository {
    backend: Backend,
}

impl CanonicalAssetRepository {
    /// Creates a repository backed by a PostgreSQL connection pool.
    pub fn new(pool: Pool) -> Self {
        Self {
            backend: Backend::Postgres(pool),
        }
    }

    /// Creates an in-memory repository for deterministic testing and offline execution.
    pub fn new_in_memory() -> Self {
        Self {
            backend: Backend::InMemory(Arc::new(RwLock::new(HashMap::new()))),
        }
    }

    /// Validates the request fields before database insertion.
    fn validate_request(req: &CreateAssetRequest) -> Result<(), ApiError> {
        if req.asset_id.trim().is_empty() {
            return Err(ApiError::ValidationError(ValidationError::InvalidParam(
                "asset_id cannot be empty".to_string(),
            )));
        }
        if req.symbol.trim().is_empty() {
            return Err(ApiError::ValidationError(ValidationError::EmptySymbol));
        }
        if req.mint_address.trim().is_empty() {
            return Err(ApiError::ValidationError(ValidationError::InvalidParam(
                "mint_address cannot be empty".to_string(),
            )));
        }
        if req.legal_issuer.trim().is_empty() {
            return Err(ApiError::ValidationError(ValidationError::InvalidParam(
                "legal_issuer cannot be empty".to_string(),
            )));
        }
        if req.custodian.trim().is_empty() {
            return Err(ApiError::ValidationError(ValidationError::InvalidParam(
                "custodian cannot be empty".to_string(),
            )));
        }
        if req.underlying_asset_identifier.trim().is_empty() {
            return Err(ApiError::ValidationError(ValidationError::InvalidParam(
                "underlying_asset_identifier cannot be empty".to_string(),
            )));
        }
        Ok(())
    }

    /// Creates a new canonical asset.
    ///
    /// Fails closed if the `mint_address` or `asset_id` is already registered.
    pub async fn create_asset(
        &self,
        req: &CreateAssetRequest,
    ) -> Result<CanonicalAssetModel, ApiError> {
        Self::validate_request(req)?;

        let initial_status = req.approval_status.unwrap_or(AssetApprovalStatus::Pending);
        let is_active = if initial_status == AssetApprovalStatus::Active {
            req.is_active
        } else {
            false
        };

        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let status_str = initial_status.to_string();
                let row = client
                    .query_one(
                        r#"
                        INSERT INTO canonical_assets (
                            asset_id, symbol, mint_address, legal_issuer, custodian,
                            underlying_asset_identifier, is_active, approval_status,
                            decimals, created_at, updated_at
                        )
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
                        RETURNING *
                        "#,
                        &[
                            &req.asset_id,
                            &req.symbol,
                            &req.mint_address,
                            &req.legal_issuer,
                            &req.custodian,
                            &req.underlying_asset_identifier,
                            &is_active,
                            &status_str,
                            &req.decimals,
                        ],
                    )
                    .await
                    .map_err(|e| {
                        let err_msg = e.to_string();
                        if err_msg.contains("uq_canonical_assets_mint")
                            || err_msg.contains("idx_canonical_assets_mint")
                            || err_msg.contains("duplicate key value violates unique constraint")
                        {
                            ApiError::BadRequest(format!(
                                "Asset with mint address '{}' or ID '{}' already exists",
                                req.mint_address, req.asset_id
                            ))
                        } else {
                            ApiError::InternalServerError(format!(
                                "Failed to insert canonical asset: {}",
                                e
                            ))
                        }
                    })?;

                Ok(CanonicalAssetModel::from(&row))
            }
            Backend::InMemory(store) => {
                let mut guard = store.write().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;

                if guard.contains_key(&req.asset_id) {
                    return Err(ApiError::BadRequest(format!(
                        "Asset with ID '{}' already exists",
                        req.asset_id
                    )));
                }

                // Check mint address uniqueness
                if guard.values().any(|a| a.mint_address == req.mint_address) {
                    return Err(ApiError::BadRequest(format!(
                        "Asset with mint address '{}' already exists",
                        req.mint_address
                    )));
                }

                let now = Utc::now();
                let model = CanonicalAssetModel {
                    asset_id: req.asset_id.clone(),
                    symbol: req.symbol.clone(),
                    mint_address: req.mint_address.clone(),
                    legal_issuer: req.legal_issuer.clone(),
                    custodian: req.custodian.clone(),
                    underlying_asset_identifier: req.underlying_asset_identifier.clone(),
                    is_active,
                    approval_status: initial_status,
                    decimals: req.decimals,
                    created_at: now,
                    updated_at: now,
                };

                guard.insert(model.asset_id.clone(), model.clone());
                Ok(model)
            }
        }
    }

    /// Alias for `create_asset`.
    pub async fn create(&self, req: &CreateAssetRequest) -> Result<CanonicalAssetModel, ApiError> {
        self.create_asset(req).await
    }

    /// Retrieves an asset by its primary key (`asset_id`).
    pub async fn get_asset_by_id(
        &self,
        asset_id: &str,
    ) -> Result<Option<CanonicalAssetModel>, ApiError> {
        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let row_opt = client
                    .query_opt(
                        "SELECT * FROM canonical_assets WHERE asset_id = $1",
                        &[&asset_id],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalServerError(format!("Failed to query asset by id: {}", e))
                    })?;

                Ok(row_opt.map(|r| CanonicalAssetModel::from(&r)))
            }
            Backend::InMemory(store) => {
                let guard = store.read().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;
                Ok(guard.get(asset_id).cloned())
            }
        }
    }

    /// Alias for `get_asset_by_id`.
    pub async fn find_by_id(
        &self,
        asset_id: &str,
    ) -> Result<Option<CanonicalAssetModel>, ApiError> {
        self.get_asset_by_id(asset_id).await
    }

    /// Alias for `get_asset_by_id`.
    pub async fn get_asset(&self, asset_id: &str) -> Result<Option<CanonicalAssetModel>, ApiError> {
        self.get_asset_by_id(asset_id).await
    }

    /// Retrieves an asset by its SPL token mint address.
    pub async fn get_asset_by_mint(
        &self,
        mint_address: &str,
    ) -> Result<Option<CanonicalAssetModel>, ApiError> {
        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let row_opt = client
                    .query_opt(
                        "SELECT * FROM canonical_assets WHERE mint_address = $1",
                        &[&mint_address],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalServerError(format!(
                            "Failed to query asset by mint: {}",
                            e
                        ))
                    })?;

                Ok(row_opt.map(|r| CanonicalAssetModel::from(&r)))
            }
            Backend::InMemory(store) => {
                let guard = store.read().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;
                Ok(guard
                    .values()
                    .find(|a| a.mint_address == mint_address)
                    .cloned())
            }
        }
    }

    /// Alias for `get_asset_by_mint`.
    pub async fn find_by_mint(
        &self,
        mint_address: &str,
    ) -> Result<Option<CanonicalAssetModel>, ApiError> {
        self.get_asset_by_mint(mint_address).await
    }

    /// Alias for `get_asset_by_mint`.
    pub async fn get_by_mint(
        &self,
        mint_address: &str,
    ) -> Result<Option<CanonicalAssetModel>, ApiError> {
        self.get_asset_by_mint(mint_address).await
    }

    /// Lists only active, Shariah-approved assets eligible for live market-data subscriptions and trading.
    ///
    /// Inactive, pending, revoked, or halted assets are strictly excluded.
    pub async fn list_active_approved_assets(&self) -> Result<Vec<CanonicalAssetModel>, ApiError> {
        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let rows = client
                    .query(
                        r#"
                        SELECT * FROM canonical_assets
                        WHERE is_active = TRUE AND approval_status = 'ACTIVE'
                        ORDER BY symbol ASC
                        "#,
                        &[],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalServerError(format!(
                            "Failed to list active approved assets: {}",
                            e
                        ))
                    })?;

                Ok(rows.iter().map(CanonicalAssetModel::from).collect())
            }
            Backend::InMemory(store) => {
                let guard = store.read().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;
                let mut list: Vec<CanonicalAssetModel> = guard
                    .values()
                    .filter(|a| a.is_active && a.approval_status == AssetApprovalStatus::Active)
                    .cloned()
                    .collect();
                list.sort_by(|a, b| a.symbol.cmp(&b.symbol));
                Ok(list)
            }
        }
    }

    /// Alias for `list_active_approved_assets`.
    pub async fn list_active_approved(&self) -> Result<Vec<CanonicalAssetModel>, ApiError> {
        self.list_active_approved_assets().await
    }

    /// Activates an asset for live market data and trading.
    ///
    /// Fails closed if the asset has not already attained `SHARIAH_APPROVED` status.
    pub async fn activate_asset(&self, asset_id: &str) -> Result<CanonicalAssetModel, ApiError> {
        let existing = self
            .get_asset_by_id(asset_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Asset '{}' not found", asset_id)))?;

        if existing.approval_status != AssetApprovalStatus::ShariahApproved
            && existing.approval_status != AssetApprovalStatus::Active
        {
            return Err(ApiError::BadRequest(format!(
                "Cannot activate asset '{}': current status is '{}', must be 'SHARIAH_APPROVED' before activation",
                asset_id, existing.approval_status
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
                        UPDATE canonical_assets
                        SET is_active = TRUE, approval_status = 'ACTIVE', updated_at = NOW()
                        WHERE asset_id = $1
                        RETURNING *
                        "#,
                        &[&asset_id],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalServerError(format!("Failed to activate asset: {}", e))
                    })?;

                Ok(CanonicalAssetModel::from(&row))
            }
            Backend::InMemory(store) => {
                let mut guard = store.write().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;

                let asset = guard
                    .get_mut(asset_id)
                    .ok_or_else(|| ApiError::NotFound(format!("Asset '{}' not found", asset_id)))?;

                asset.is_active = true;
                asset.approval_status = AssetApprovalStatus::Active;
                asset.updated_at = Utc::now();
                Ok(asset.clone())
            }
        }
    }

    /// Alias for `activate_asset`.
    pub async fn activate(&self, asset_id: &str) -> Result<CanonicalAssetModel, ApiError> {
        self.activate_asset(asset_id).await
    }

    /// Deactivates an asset, immediately removing it from live trading and market data feeds.
    pub async fn deactivate_asset(&self, asset_id: &str) -> Result<CanonicalAssetModel, ApiError> {
        let _existing = self
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
                        UPDATE canonical_assets
                        SET is_active = FALSE, approval_status = 'DEACTIVATED', updated_at = NOW()
                        WHERE asset_id = $1
                        RETURNING *
                        "#,
                        &[&asset_id],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalServerError(format!("Failed to deactivate asset: {}", e))
                    })?;

                Ok(CanonicalAssetModel::from(&row))
            }
            Backend::InMemory(store) => {
                let mut guard = store.write().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;

                let asset = guard
                    .get_mut(asset_id)
                    .ok_or_else(|| ApiError::NotFound(format!("Asset '{}' not found", asset_id)))?;

                asset.is_active = false;
                asset.approval_status = AssetApprovalStatus::Deactivated;
                asset.updated_at = Utc::now();
                Ok(asset.clone())
            }
        }
    }

    /// Alias for `deactivate_asset`.
    pub async fn deactivate(&self, asset_id: &str) -> Result<CanonicalAssetModel, ApiError> {
        self.deactivate_asset(asset_id).await
    }

    /// Transitions an asset's lifecycle state according to defined state machine rules.
    pub async fn update_approval_status(
        &self,
        asset_id: &str,
        target_status: AssetApprovalStatus,
    ) -> Result<CanonicalAssetModel, ApiError> {
        let existing = self
            .get_asset_by_id(asset_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Asset '{}' not found", asset_id)))?;

        if !existing.approval_status.can_transition_to(target_status) {
            return Err(ApiError::BadRequest(format!(
                "Invalid status transition for '{}' from '{}' to '{}'",
                asset_id, existing.approval_status, target_status
            )));
        }

        let is_active = target_status == AssetApprovalStatus::Active;

        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let status_str = target_status.to_string();
                let row = client
                    .query_one(
                        r#"
                        UPDATE canonical_assets
                        SET approval_status = $2, is_active = $3, updated_at = NOW()
                        WHERE asset_id = $1
                        RETURNING *
                        "#,
                        &[&asset_id, &status_str, &is_active],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalServerError(format!(
                            "Failed to update asset status: {}",
                            e
                        ))
                    })?;

                Ok(CanonicalAssetModel::from(&row))
            }
            Backend::InMemory(store) => {
                let mut guard = store.write().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;

                let asset = guard
                    .get_mut(asset_id)
                    .ok_or_else(|| ApiError::NotFound(format!("Asset '{}' not found", asset_id)))?;

                asset.approval_status = target_status;
                asset.is_active = is_active;
                asset.updated_at = Utc::now();
                Ok(asset.clone())
            }
        }
    }

    /// Filters assets by their approval lifecycle status.
    pub async fn list_by_status(
        &self,
        status: AssetApprovalStatus,
    ) -> Result<Vec<CanonicalAssetModel>, ApiError> {
        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let status_str = status.to_string();
                let rows = client
                    .query(
                        r#"
                        SELECT * FROM canonical_assets
                        WHERE approval_status = $1
                        ORDER BY symbol ASC
                        "#,
                        &[&status_str],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalServerError(format!(
                            "Failed to filter assets by status: {}",
                            e
                        ))
                    })?;

                Ok(rows.iter().map(CanonicalAssetModel::from).collect())
            }
            Backend::InMemory(store) => {
                let guard = store.read().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;
                let mut list: Vec<CanonicalAssetModel> = guard
                    .values()
                    .filter(|a| a.approval_status == status)
                    .cloned()
                    .collect();
                list.sort_by(|a, b| a.symbol.cmp(&b.symbol));
                Ok(list)
            }
        }
    }

    /// Lists all registered assets ordered by creation time.
    pub async fn list_all(&self) -> Result<Vec<CanonicalAssetModel>, ApiError> {
        match &self.backend {
            Backend::Postgres(pool) => {
                let client = pool.get().await.map_err(|e| {
                    ApiError::InternalServerError(format!("Database connection failed: {}", e))
                })?;

                let rows = client
                    .query(
                        "SELECT * FROM canonical_assets ORDER BY created_at DESC",
                        &[],
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalServerError(format!("Failed to list all assets: {}", e))
                    })?;

                Ok(rows.iter().map(CanonicalAssetModel::from).collect())
            }
            Backend::InMemory(store) => {
                let guard = store.read().map_err(|e| {
                    ApiError::InternalServerError(format!("Lock acquisition failed: {}", e))
                })?;
                let mut list: Vec<CanonicalAssetModel> = guard.values().cloned().collect();
                list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                Ok(list)
            }
        }
    }
}
