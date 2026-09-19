//! Shariah-gated Asset Registry security gate.
//!
//! # Core Security Invariant: Existence != Tradeability
//! An asset cannot be traded or held in vaults merely because an SPL token mint
//! or price feed exists. An asset must pass an authoritative gate verifying:
//! 1. Valid Asset identity and token details
//! 2. Valid and active [`OwnershipRecord`](crate::shariah::OwnershipRecord)
//! 3. Permissible [`BusinessCategory`](crate::shariah::BusinessCategory)
//! 4. Quantitative financial metrics passing [`ScreeningPolicy`](crate::shariah::ScreeningPolicy)
//! 5. Current, non-expired review window
//! 6. Authentic cryptographic evidence hash
//!
//! Assets with [`ShariahStatus::Pending`], [`ShariahStatus::Rejected`],
//! [`ShariahStatus::Expired`], or [`ShariahStatus::Revoked`] are strictly barred from trading.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use super::eligibility::ShariahEligibility;
use super::ownership::{validate_ownership, OwnershipRecord};
use super::policy::ScreeningPolicy;
use super::screening::{
    screen_business_activity, screen_financial_metrics, BusinessCategory, ShariahFinancialMetrics,
};
use super::types::{ShariahRejectionReason, ShariahStatus};
use crate::asset::{Asset, AssetStatus};

/// Registry errors returned during asset admission or trade authorization.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RegistryError {
    #[error("Asset validation failed: {0}")]
    InvalidAsset(String),

    #[error("Ownership verification failed: {0}")]
    InvalidOwnership(String),

    #[error("Asset '{symbol}' rejected by Shariah screening: {reason}")]
    ShariahRejected {
        symbol: String,
        reason: ShariahRejectionReason,
    },

    #[error("Asset '{0}' screening review has expired")]
    ReviewExpired(String),

    #[error("Asset '{0}' is not tradeable: status is {1}")]
    NotTradeable(String, ShariahStatus),

    #[error("Duplicate asset in registry: {0}")]
    DuplicateAsset(String),

    #[error("Asset '{0}' not found in registry")]
    AssetNotFound(String),
}

/// Fully admitted, Shariah-screened asset registered with verified compliance credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredAsset {
    /// Canonical asset identity, token specifications, and market data routing.
    pub asset: Asset,
    /// Verified Shariah qualification and audit certification.
    pub eligibility: ShariahEligibility,
}

impl RegisteredAsset {
    /// Returns true if this asset is currently permissible for spot execution at timestamp `now`.
    pub fn is_tradeable(&self, now: i64) -> bool {
        is_tradeable(self, now)
    }

    /// Convenience getter for Shariah eligibility.
    pub fn eligibility(&self) -> &ShariahEligibility {
        &self.eligibility
    }
}

/// Standalone accessor retrieving Shariah eligibility from a [`RegisteredAsset`].
pub fn get_eligibility(registered: &RegisteredAsset) -> &ShariahEligibility {
    &registered.eligibility
}

/// Evaluates whether a [`RegisteredAsset`] is actively tradeable at timestamp `now`.
///
/// Must satisfy:
/// 1. `status == ShariahStatus::Approved`
/// 2. `ownership_verified == true`
/// 3. `now >= reviewed_at` and `now < expires_at`
/// 4. `asset.provider.status == AssetStatus::Active`
/// 5. Passes all internal eligibility criteria
pub fn is_tradeable(registered: &RegisteredAsset, now: i64) -> bool {
    if registered.eligibility.status != ShariahStatus::Approved {
        return false;
    }
    if !registered.eligibility.ownership_verified {
        return false;
    }
    if now < registered.eligibility.reviewed_at || now >= registered.eligibility.expires_at {
        return false;
    }
    if registered.asset.provider.status != AssetStatus::Active {
        return false;
    }
    registered.eligibility.is_currently_eligible(now)
}

/// Registers an asset evaluating all security gate requirements at `ownership.verified_at`.
pub fn register_asset(
    asset: Asset,
    ownership: OwnershipRecord,
    financials: ShariahFinancialMetrics,
    business: BusinessCategory,
    policy: &ScreeningPolicy,
) -> Result<RegisteredAsset, RegistryError> {
    let now = ownership.verified_at;
    register_asset_at(asset, ownership, financials, business, policy, now)
}

/// Registers an asset evaluating all security gate requirements at explicit timestamp `now`.
pub fn register_asset_at(
    asset: Asset,
    ownership: OwnershipRecord,
    financials: ShariahFinancialMetrics,
    business: BusinessCategory,
    policy: &ScreeningPolicy,
    now: i64,
) -> Result<RegisteredAsset, RegistryError> {
    let symbol = asset.symbol().to_string();

    // 1. Asset identity validation
    if symbol.trim().is_empty() {
        return Err(RegistryError::InvalidAsset(
            "asset symbol cannot be empty".to_string(),
        ));
    }
    if asset.id().trim().is_empty() {
        return Err(RegistryError::InvalidAsset(
            "asset_id cannot be empty".to_string(),
        ));
    }
    if asset.mint().trim().is_empty() {
        return Err(RegistryError::InvalidAsset(
            "token mint cannot be empty".to_string(),
        ));
    }

    // 2. Ownership record structural validation
    if let Err(err) = validate_ownership(&ownership) {
        return Err(RegistryError::InvalidOwnership(err.to_string()));
    }

    // 3. Temporal validity check
    if now < ownership.verified_at {
        return Err(RegistryError::InvalidOwnership(
            "evaluation timestamp precedes verification timestamp".to_string(),
        ));
    }
    if now >= ownership.expires_at {
        return Err(RegistryError::ReviewExpired(symbol));
    }

    // 4. Evidence hash non-empty check
    if ownership.evidence_hash.trim().is_empty() {
        return Err(RegistryError::InvalidOwnership(
            "evidence hash cannot be empty".to_string(),
        ));
    }

    // 5. Qualitative business sector filter
    if let Err(reason) = screen_business_activity(&business) {
        return Err(RegistryError::ShariahRejected { symbol, reason });
    }

    // 6. Quantitative financial screening benchmarks
    if let Err(reason) = screen_financial_metrics(&financials, policy) {
        return Err(RegistryError::ShariahRejected { symbol, reason });
    }

    // Construct confirmed ShariahEligibility record
    let eligibility = ShariahEligibility {
        status: ShariahStatus::Approved,
        standard: policy.standard.clone(),
        business_activity_approved: true,
        debt_ratio_bps: financials.debt_ratio_bps,
        interest_bearing_cash_bps: financials.interest_bearing_cash_ratio_bps,
        impure_income_bps: financials.impure_income_ratio_bps,
        ownership_verified: true,
        evidence_hash: ownership.evidence_hash,
        reviewed_at: ownership.verified_at,
        expires_at: ownership.expires_at,
        policy_version: policy.policy_version.clone(),
    };

    Ok(RegisteredAsset { asset, eligibility })
}

/// Authoritative in-memory registry holding Shariah-screened, verified assets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShariahAssetRegistry {
    assets: HashMap<String, RegisteredAsset>,
}

impl ShariahAssetRegistry {
    /// Creates an empty asset registry.
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    /// Registers an asset through the full Shariah security gate at timestamp `now`.
    pub fn register(
        &mut self,
        asset: Asset,
        ownership: OwnershipRecord,
        financials: ShariahFinancialMetrics,
        business: BusinessCategory,
        policy: &ScreeningPolicy,
        now: i64,
    ) -> Result<RegisteredAsset, RegistryError> {
        let asset_id = asset.id().to_string();
        if self.assets.contains_key(&asset_id) {
            return Err(RegistryError::DuplicateAsset(asset_id));
        }

        let registered = register_asset_at(asset, ownership, financials, business, policy, now)?;
        self.assets.insert(asset_id, registered.clone());
        Ok(registered)
    }

    /// Retrieves the Shariah eligibility record for an asset if present.
    pub fn get_eligibility(&self, asset_id: &str) -> Option<&ShariahEligibility> {
        self.assets.get(asset_id).map(|r| &r.eligibility)
    }

    /// Retrieves the full [`RegisteredAsset`] by canonical ID or ticker symbol.
    pub fn get_asset(&self, query: &str) -> Option<&RegisteredAsset> {
        if let Some(asset) = self.assets.get(query) {
            return Some(asset);
        }
        self.assets
            .values()
            .find(|r| r.asset.symbol().eq_ignore_ascii_case(query))
    }

    /// Evaluates if an asset is registered and actively tradeable at timestamp `now`.
    pub fn is_tradeable(&self, query: &str, now: i64) -> bool {
        self.get_asset(query)
            .map(|r| is_tradeable(r, now))
            .unwrap_or(false)
    }

    /// Retrieves a mutable reference to a [`RegisteredAsset`] by canonical ID or ticker symbol.
    pub fn get_asset_mut(&mut self, query: &str) -> Option<&mut RegisteredAsset> {
        if self.assets.contains_key(query) {
            return self.assets.get_mut(query);
        }
        self.assets
            .values_mut()
            .find(|r| r.asset.symbol().eq_ignore_ascii_case(query))
    }

    /// Revokes an asset's approval status (e.g. following a board decree or delisting).
    pub fn revoke_asset(&mut self, query: &str) -> Result<(), RegistryError> {
        let entry = self
            .get_asset_mut(query)
            .ok_or_else(|| RegistryError::AssetNotFound(query.to_string()))?;

        entry.eligibility.status = ShariahStatus::Revoked;
        Ok(())
    }

    /// Marks an asset's status as expired.
    pub fn expire_asset(&mut self, query: &str) -> Result<(), RegistryError> {
        let entry = self
            .get_asset_mut(query)
            .ok_or_else(|| RegistryError::AssetNotFound(query.to_string()))?;

        entry.eligibility.status = ShariahStatus::Expired;
        Ok(())
    }
}
