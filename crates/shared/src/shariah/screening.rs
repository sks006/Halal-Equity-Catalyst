//! Deterministic Shariah screening engine.
//!
//! # Separation of Concerns: Eligibility Screening vs. Dividend Purification
//! In Islamic finance governance (e.g. AAOIFI Standard No. 21):
//! - **Eligibility Screening** evaluates whether an equity security is permissible for spot ownership
//!   and trading within a portfolio, based on qualitative sector filters, quantitative capital
//!   structure thresholds, and custodial ownership backing.
//! - **Dividend Purification** is a distinct post-settlement accounting mechanism. When an approved
//!   portfolio company distributes dividends, the exact pro-rata percentage of revenue derived from
//!   incidental or non-operating impure activities (e.g. 1.5% interest on operational bank deposits)
//!   must be segregated and donated to charity without capital gain or tax offset.
//!
//! Eligibility screening does NOT invent or deduct arbitrary purification fees; it deterministically
//! approves or rejects an asset based strictly on configured policy benchmarks.
//!
//! # Threshold Inclusivity Definition
//! All quantitative financial thresholds in [`ScreeningPolicy`](crate::shariah::ScreeningPolicy)
//! define **inclusive maximum permissible upper bounds**:
//! - Ratio $\le$ Limit: **COMPLIANT (Pass)**
//! - Ratio $>$ Limit: **NON-COMPLIANT (Fail)**
//!
//! Example (3,000 bps debt limit):
//! - 2,999 bps (limit - 1): **Approved**
//! - 3,000 bps (limit): **Approved**
//! - 3,001 bps (limit + 1): **Rejected (`ExcessDebt`)**

use serde::{Deserialize, Serialize};
use std::fmt;

use super::ownership::OwnershipRecord;
use super::policy::ScreeningPolicy;
use super::types::ShariahRejectionReason;

/// Broad industry and economic sector classification for qualitative Shariah filtering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BusinessCategory {
    Technology,
    Healthcare,
    Manufacturing,
    ConventionalFinance,
    Alcohol,
    Gambling,
    Tobacco,
    Weapons,
    AdultEntertainment,
    Other(String),
}

impl BusinessCategory {
    /// Returns true if this sector is impermissible in Islamic jurisprudence.
    pub fn is_prohibited(&self) -> bool {
        matches!(
            self,
            Self::ConventionalFinance
                | Self::Alcohol
                | Self::Gambling
                | Self::Tobacco
                | Self::Weapons
                | Self::AdultEntertainment
        )
    }
}

impl fmt::Display for BusinessCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Technology => write!(f, "Technology"),
            Self::Healthcare => write!(f, "Healthcare"),
            Self::Manufacturing => write!(f, "Manufacturing"),
            Self::ConventionalFinance => write!(f, "Conventional Finance"),
            Self::Alcohol => write!(f, "Alcohol"),
            Self::Gambling => write!(f, "Gambling"),
            Self::Tobacco => write!(f, "Tobacco"),
            Self::Weapons => write!(f, "Weapons"),
            Self::AdultEntertainment => write!(f, "Adult Entertainment"),
            Self::Other(name) => write!(f, "Other({})", name),
        }
    }
}

/// Normalized financial screening metrics evaluated against a [`ScreeningPolicy`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShariahFinancialMetrics {
    /// Trailing total interest-bearing debt relative to 12-month trailing market capitalization, in basis points.
    pub debt_ratio_bps: u16,
    /// Trailing cash and interest-bearing deposits relative to 12-month trailing market capitalization, in basis points.
    pub interest_bearing_cash_ratio_bps: u16,
    /// Incidental or non-operating impermissible revenue relative to total revenue, in basis points.
    pub impure_income_ratio_bps: u16,
}

/// Outcome of deterministic Shariah compliance screening.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreeningResult {
    /// Asset satisfies all qualitative, quantitative, and custodial ownership criteria.
    Approved,
    /// Asset fails screening with explicit deterministic cause.
    Rejected { reason: ShariahRejectionReason },
}

impl ScreeningResult {
    /// Returns true if the asset was approved.
    #[inline]
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved)
    }

    /// Returns the rejection reason if the asset was rejected.
    #[inline]
    pub fn rejection_reason(&self) -> Option<&ShariahRejectionReason> {
        match self {
            Self::Approved => None,
            Self::Rejected { reason } => Some(reason),
        }
    }
}

/// Evaluates core business activity permissibility.
///
/// Returns `Ok(())` for permissible sectors (Technology, Healthcare, Manufacturing, etc.),
/// or `Err(ShariahRejectionReason::ProhibitedBusiness)` for prohibited sectors.
pub fn screen_business_activity(
    category: &BusinessCategory,
) -> Result<(), ShariahRejectionReason> {
    if category.is_prohibited() {
        return Err(ShariahRejectionReason::ProhibitedBusiness);
    }
    Ok(())
}

/// Evaluates quantitative capital structure and revenue purity ratios against a [`ScreeningPolicy`].
///
/// # Inclusivity Rules
/// - Ratios are evaluated inclusively ($\le \text{limit}$).
/// - If `debt_ratio_bps > policy.debt_limit_bps`: returns `Err(ShariahRejectionReason::ExcessDebt)`.
/// - If `interest_bearing_cash_ratio_bps > policy.interest_bearing_cash_limit_bps`: returns `Err(ShariahRejectionReason::ExcessInterestBearingCash)`.
/// - If `impure_income_ratio_bps > policy.impure_income_limit_bps`: returns `Err(ShariahRejectionReason::ExcessImpureIncome)`.
pub fn screen_financial_metrics(
    metrics: &ShariahFinancialMetrics,
    policy: &ScreeningPolicy,
) -> Result<(), ShariahRejectionReason> {
    if metrics.debt_ratio_bps > policy.debt_limit_bps.as_bps() {
        return Err(ShariahRejectionReason::ExcessDebt);
    }

    if metrics.interest_bearing_cash_ratio_bps > policy.interest_bearing_cash_limit_bps.as_bps() {
        return Err(ShariahRejectionReason::ExcessInterestBearingCash);
    }

    if metrics.impure_income_ratio_bps > policy.impure_income_limit_bps.as_bps() {
        return Err(ShariahRejectionReason::ExcessImpureIncome);
    }

    Ok(())
}

/// Fully deterministic asset screening engine evaluating ownership, qualitative business,
/// and quantitative metrics against the active [`ScreeningPolicy`].
///
/// Execution order:
/// 1. Ownership & custody verification at timestamp `now`.
/// 2. Core business activity permissibility.
/// 3. Quantitative financial ratio thresholds.
pub fn screen_asset(
    category: &BusinessCategory,
    metrics: &ShariahFinancialMetrics,
    ownership: &OwnershipRecord,
    policy: &ScreeningPolicy,
    now: i64,
) -> ScreeningResult {
    // 1. Custody and beneficial ownership verification
    if !ownership.verified || ownership.validate().is_err() {
        return ScreeningResult::Rejected {
            reason: ShariahRejectionReason::OwnershipUnverified,
        };
    }
    if !ownership.is_current(now) {
        return ScreeningResult::Rejected {
            reason: ShariahRejectionReason::ReviewExpired,
        };
    }

    // 2. Qualitative business sector filter
    if let Err(reason) = screen_business_activity(category) {
        return ScreeningResult::Rejected { reason };
    }

    // 3. Quantitative financial benchmarks
    if let Err(reason) = screen_financial_metrics(metrics, policy) {
        return ScreeningResult::Rejected { reason };
    }

    ScreeningResult::Approved
}
