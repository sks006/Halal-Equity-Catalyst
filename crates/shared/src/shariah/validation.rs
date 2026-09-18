//! Shariah screening deterministic validation and compliance evaluation engine.

use serde::{Deserialize, Serialize};

use crate::types::BasisPoints;
use crate::validation::ValidationError;
use super::policy::ScreeningPolicy;
use super::types::{ShariahRejectionReason, ShariahStatus};

/// Financial and qualitative evaluation inputs submitted to screen an asset against a policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreeningAssessmentInput {
    /// True if the primary corporate business activity is permissible under Shariah principles.
    pub has_permissible_business: bool,
    /// True if underlying 1:1 shares are held in bankruptcy-remote custody / SPV.
    pub is_ownership_verified: bool,
    /// True if the token represents an unbacked synthetic, CFD, or derivative contract.
    pub is_synthetic: bool,
    /// True if sufficient audited financial statements and disclosures are present.
    pub has_sufficient_evidence: bool,
    /// True if the screening evaluation is within its valid periodic audit window.
    pub is_review_current: bool,
    /// Trailing interest-bearing debt relative to market capitalization, in basis points.
    pub debt_ratio_bps: BasisPoints,
    /// Trailing cash and interest-bearing deposits relative to market capitalization, in basis points.
    pub interest_bearing_cash_ratio_bps: BasisPoints,
    /// Non-operating or incidental impure revenue relative to total revenue, in basis points.
    pub impure_income_ratio_bps: BasisPoints,
}

impl Default for ScreeningAssessmentInput {
    fn default() -> Self {
        Self {
            has_permissible_business: true,
            is_ownership_verified: true,
            is_synthetic: false,
            has_sufficient_evidence: true,
            is_review_current: true,
            debt_ratio_bps: BasisPoints::ZERO,
            interest_bearing_cash_ratio_bps: BasisPoints::ZERO,
            impure_income_ratio_bps: BasisPoints::ZERO,
        }
    }
}

/// Evaluates screening metrics deterministically against a specified [`ScreeningPolicy`].
///
/// Returns the resulting [`ShariahStatus`] alongside all applicable [`ShariahRejectionReason`]s.
pub fn evaluate_shariah_compliance(
    policy: &ScreeningPolicy,
    input: &ScreeningAssessmentInput,
) -> (ShariahStatus, Vec<ShariahRejectionReason>) {
    let mut reasons = Vec::new();

    // 1. Qualitative Business Activity Filter
    if !input.has_permissible_business {
        reasons.push(ShariahRejectionReason::ProhibitedBusiness);
    }

    // 2. Structural & Ownership Integrity Filters
    if input.is_synthetic {
        reasons.push(ShariahRejectionReason::SyntheticExposure);
    }
    if !input.is_ownership_verified {
        reasons.push(ShariahRejectionReason::OwnershipUnverified);
    }

    // 3. Quantitative Financial Ratio Benchmarks (bound to policy version)
    if input.debt_ratio_bps.0 > policy.debt_limit_bps.0 {
        reasons.push(ShariahRejectionReason::ExcessDebt);
    }
    if input.interest_bearing_cash_ratio_bps.0 > policy.interest_bearing_cash_limit_bps.0 {
        reasons.push(ShariahRejectionReason::ExcessInterestBearingCash);
    }
    if input.impure_income_ratio_bps.0 > policy.impure_income_limit_bps.0 {
        reasons.push(ShariahRejectionReason::ExcessImpureIncome);
    }

    // 4. Evidence and Audit Validity Lifecycle
    if !input.has_sufficient_evidence {
        reasons.push(ShariahRejectionReason::MissingEvidence);
    }
    if !input.is_review_current {
        reasons.push(ShariahRejectionReason::ReviewExpired);
    }

    // Determine final status
    if reasons.is_empty() {
        (ShariahStatus::Approved, reasons)
    } else {
        // If missing evidence is the sole reason (no substantive disqualifications), status is Pending
        let has_substantive_failure = reasons.iter().any(|r| {
            matches!(
                r,
                ShariahRejectionReason::ProhibitedBusiness
                    | ShariahRejectionReason::SyntheticExposure
                    | ShariahRejectionReason::OwnershipUnverified
                    | ShariahRejectionReason::ExcessDebt
                    | ShariahRejectionReason::ExcessInterestBearingCash
                    | ShariahRejectionReason::ExcessImpureIncome
                    | ShariahRejectionReason::Other(_)
            )
        });

        if !has_substantive_failure {
            if reasons.iter().any(|r| matches!(r, ShariahRejectionReason::MissingEvidence)) {
                (ShariahStatus::Pending, reasons)
            } else if reasons.iter().any(|r| matches!(r, ShariahRejectionReason::ReviewExpired)) {
                (ShariahStatus::Expired, reasons)
            } else {
                (ShariahStatus::Rejected, reasons)
            }
        } else {
            (ShariahStatus::Rejected, reasons)
        }
    }
}

/// Validates that a [`ScreeningPolicy`] has sound parameter bounds.
pub fn validate_screening_policy(policy: &ScreeningPolicy) -> Result<(), ValidationError> {
    policy.validate()
}
