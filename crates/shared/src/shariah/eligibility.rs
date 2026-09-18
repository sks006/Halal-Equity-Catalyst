//! First-class Shariah eligibility domain model.
//!
//! # Separation of Concerns
//! - [`Asset`](crate::asset::Asset) captures immutable identity, token mint, and market data wiring.
//! - [`ShariahEligibility`] captures whether an asset is currently permissible under a configured
//!   screening policy, evidence integrity records, and periodic review validity windows.
//!
//! # Evidence Integrity vs. Truth Notice
//! The `evidence_hash` field stores a cryptographic digest (e.g. SHA-256) of the audited financial
//! filings, supervisory board review dossier, and custodial ownership certifications.
//!
//! **Important Distinction**:
//! `evidence_hash != proof that the claim is true`.
//! The cryptographic hash proves only that the specific evidence examined during review has not
//! been silently altered, tampered with, or substituted after the screening determination was recorded.

use serde::{Deserialize, Serialize};

use super::policy::ScreeningPolicy;
use super::types::{ScreeningStandard, ShariahRejectionReason, ShariahStatus};

/// Comprehensive Shariah compliance qualification record for an individual asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShariahEligibility {
    /// Current compliance qualification status.
    pub status: ShariahStatus,
    /// Shariah screening framework standard applied.
    pub standard: ScreeningStandard,
    /// Qualitative certification that primary business activity does not involve impermissible sectors.
    pub business_activity_approved: bool,
    /// Trailing interest-bearing debt ratio in basis points (e.g., 2,850 = 28.50%).
    pub debt_ratio_bps: u16,
    /// Trailing cash and interest-bearing deposits ratio in basis points.
    pub interest_bearing_cash_bps: u16,
    /// Revenue derived from incidental/impure non-operating sources in basis points.
    pub impure_income_bps: u16,
    /// Verification that underlying 1:1 shares are held in bankruptcy-remote custody or SPV.
    pub ownership_verified: bool,
    /// Cryptographic digest of the reviewed audit and filing dossier proving tamper-evidence.
    ///
    /// Note: Proves evidence immutability, not that external corporate filings are objectively true.
    pub evidence_hash: String,
    /// Unix timestamp (seconds) when the screening review was performed.
    pub reviewed_at: i64,
    /// Unix timestamp (seconds) when the current screening determination expires.
    pub expires_at: i64,
    /// Version identifier of the screening policy applied (e.g., "v1.0", "aaoifi-21-2024").
    pub policy_version: String,
}

impl ShariahEligibility {
    /// Returns the active default [`ScreeningPolicy`] configured for this eligibility record's
    /// [`ScreeningStandard`] and `policy_version`.
    pub fn active_policy(&self) -> ScreeningPolicy {
        match &self.standard {
            ScreeningStandard::BoardApprovedV1 => ScreeningPolicy {
                standard: ScreeningStandard::BoardApprovedV1,
                policy_version: self.policy_version.clone(),
                ..ScreeningPolicy::board_approved_v1()
            },
            ScreeningStandard::Aaoifi21 => ScreeningPolicy::aaoifi_21(&self.policy_version),
            ScreeningStandard::Custom(name) => ScreeningPolicy {
                standard: ScreeningStandard::Custom(name.clone()),
                policy_version: self.policy_version.clone(),
                ..ScreeningPolicy::board_approved_v1()
            },
        }
    }

    /// Evaluates whether the asset is currently eligible for spot trading at timestamp `now`
    /// under its associated standard and policy version.
    ///
    /// # Eligibility Rules
    /// 1. `status` must be [`ShariahStatus::Approved`].
    /// 2. `ownership_verified` must be `true`.
    /// 3. `now >= reviewed_at` (audit date has occurred; no future clock skew).
    /// 4. `now < expires_at` (current audit has not lapsed).
    /// 5. `evidence_hash` must not be empty.
    /// 6. `business_activity_approved` must be `true`.
    /// 7. All financial screening ratios must satisfy the active [`ScreeningPolicy`].
    pub fn is_currently_eligible(&self, now: i64) -> bool {
        let policy = self.active_policy();
        self.is_currently_eligible_under_policy(&policy, now)
    }

    /// Evaluates whether the asset is currently eligible at timestamp `now` under a specifically
    /// supplied [`ScreeningPolicy`].
    pub fn is_currently_eligible_under_policy(&self, policy: &ScreeningPolicy, now: i64) -> bool {
        self.validate_eligibility_under_policy(policy, now).is_ok()
    }

    /// Deterministically validates all eligibility rules against the active default policy,
    /// returning `Ok(())` on success or an error containing every applicable [`ShariahRejectionReason`].
    pub fn validate_eligibility(&self, now: i64) -> Result<(), Vec<ShariahRejectionReason>> {
        let policy = self.active_policy();
        self.validate_eligibility_under_policy(&policy, now)
    }

    /// Deterministically validates all eligibility rules against a specifically supplied [`ScreeningPolicy`],
    /// returning `Ok(())` on success or an error containing every applicable [`ShariahRejectionReason`].
    pub fn validate_eligibility_under_policy(
        &self,
        policy: &ScreeningPolicy,
        now: i64,
    ) -> Result<(), Vec<ShariahRejectionReason>> {
        let mut reasons = Vec::new();

        // 1. Status rule: Must be Approved
        if self.status != ShariahStatus::Approved {
            match self.status {
                ShariahStatus::Pending => reasons.push(ShariahRejectionReason::MissingEvidence),
                ShariahStatus::Expired => reasons.push(ShariahRejectionReason::ReviewExpired),
                ShariahStatus::Rejected => reasons.push(ShariahRejectionReason::Other(
                    "Asset status is Rejected".to_string(),
                )),
                ShariahStatus::Revoked => reasons.push(ShariahRejectionReason::Other(
                    "Asset status is Revoked".to_string(),
                )),
                ShariahStatus::Approved => {}
            }
        }

        // 2. Ownership verification rule
        if !self.ownership_verified {
            reasons.push(ShariahRejectionReason::OwnershipUnverified);
        }

        // 3. Evidence integrity rule: Hash must not be empty
        if self.evidence_hash.trim().is_empty() {
            reasons.push(ShariahRejectionReason::MissingEvidence);
        }

        // 4. Temporal validity rules
        if now < self.reviewed_at {
            reasons.push(ShariahRejectionReason::Other(
                "Evaluation timestamp precedes review timestamp".to_string(),
            ));
        }
        if now >= self.expires_at {
            reasons.push(ShariahRejectionReason::ReviewExpired);
        }

        // 5. Qualitative business activity rule
        if !self.business_activity_approved {
            reasons.push(ShariahRejectionReason::ProhibitedBusiness);
        }

        // 6. Quantitative financial screening values vs policy limits
        if self.debt_ratio_bps > policy.debt_limit_bps.as_bps() {
            reasons.push(ShariahRejectionReason::ExcessDebt);
        }
        if self.interest_bearing_cash_bps > policy.interest_bearing_cash_limit_bps.as_bps() {
            reasons.push(ShariahRejectionReason::ExcessInterestBearingCash);
        }
        if self.impure_income_bps > policy.impure_income_limit_bps.as_bps() {
            reasons.push(ShariahRejectionReason::ExcessImpureIncome);
        }

        if reasons.is_empty() {
            Ok(())
        } else {
            Err(reasons)
        }
    }
}
