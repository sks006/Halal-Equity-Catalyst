//! Core Shariah screening domain types for Equity Catalyst.
//!
//! # Regulatory & Compliance Architecture Notice
//! Equity Catalyst is engineered from first principles for Shariah compliance
//! (100% equity spot settlement, 0% leverage, zero interest, and custodial ownership backing).
//! In accordance with Islamic finance governance, software approval certifies compliance with
//! algorithmic policy rules, but does not substitute for formal certification by a qualified
//! Shariah supervisory board.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Compliance and qualification status of an asset within the Shariah screening engine.
///
/// # Status Semantics
/// - `Pending`: Awaiting initial qualitative/financial screening or verification of evidence.
///   Assets in `Pending` status cannot be held or traded in Shariah vaults.
/// - `Approved`: Screened and confirmed compliant under the designated `ScreeningPolicy`.
///   Eligible for spot vault inclusion and execution. Algorithmic software approval is distinct
///   from formal religious board certification.
/// - `Rejected`: Failed one or more mandatory screening criteria (business activity, financial
///   ratios, unverified custody, or synthetic structure). Strictly barred from trading.
/// - `Expired`: The periodic compliance review window (e.g. quarterly earnings or annual audit)
///   has elapsed without recertification. Trading is halted or restricted to divestment.
/// - `Revoked`: A previously approved status that has been proactively invalidated due to a
///   material corporate breach, delisting, or board disqualification. Requires immediate orderly exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShariahStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Revoked,
}

impl ShariahStatus {
    /// Returns true if the asset is currently approved for spot trading.
    #[inline]
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved)
    }

    /// Returns true if the asset is rejected due to Shariah non-compliance.
    #[inline]
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected)
    }

    /// Returns true if the asset is awaiting review or evidence.
    #[inline]
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns true if the asset screening has expired and needs re-audit.
    #[inline]
    pub fn is_expired(&self) -> bool {
        matches!(self, Self::Expired)
    }

    /// Returns true if the asset approval was explicitly revoked.
    #[inline]
    pub fn is_revoked(&self) -> bool {
        matches!(self, Self::Revoked)
    }

    /// Returns true if new purchase allocations are permissible (only when Approved).
    #[inline]
    pub fn can_trade(&self) -> bool {
        matches!(self, Self::Approved)
    }
}

impl fmt::Display for ShariahStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Approved => write!(f, "Approved"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Expired => write!(f, "Expired"),
            Self::Revoked => write!(f, "Revoked"),
        }
    }
}

/// Shariah screening framework standard applied to evaluate an asset.
///
/// External standards (e.g. AAOIFI vs. DJIM vs. FTSE Islamic vs. SAC Malaysia)
/// employ differing financial ratio thresholds (e.g. 30% vs 33% debt limits)
/// and market cap calculation windows. Standards are decoupled from fixed global constants
/// and parameterized per policy version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScreeningStandard {
    /// Platform default baseline conservative standard approved by the initial supervisory board.
    BoardApprovedV1,
    /// Accounting and Auditing Organization for Islamic Financial Institutions Standard No. 21.
    Aaoifi21,
    /// Bespoke supervisory board, institutional, or regional standard (e.g. "DJIM", "SAC-SC-Malaysia").
    Custom(String),
}

impl fmt::Display for ScreeningStandard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BoardApprovedV1 => write!(f, "BoardApprovedV1"),
            Self::Aaoifi21 => write!(f, "Aaoifi21"),
            Self::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

/// Specific qualitative, quantitative, or structural reason for Shariah screening rejection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShariahRejectionReason {
    /// Core business activity violates Islamic principles (conventional banking, alcohol, gambling,
    /// tobacco, adult entertainment, weapons, etc.).
    ProhibitedBusiness,
    /// Total interest-bearing debt exceeds the policy limit relative to market capitalization.
    ExcessDebt,
    /// Total cash and interest-bearing deposits exceed the policy limit relative to market capitalization.
    ExcessInterestBearingCash,
    /// Impermissible/non-operating income exceeds the policy threshold relative to total revenue.
    ExcessImpureIncome,
    /// Direct beneficial ownership, bankruptcy-remote SPV custody, or statutory shares cannot be verified.
    OwnershipUnverified,
    /// Instrument is a synthetic derivative, CFD, or debt-backed obligation lacking genuine 1:1 asset backing.
    SyntheticExposure,
    /// Insufficient audit disclosures, broken financial data feeds, or missing required filings.
    MissingEvidence,
    /// Compliance validity period elapsed without periodic recertification.
    ReviewExpired,
    /// Other specific scholar- or board-mandated rejection criteria.
    Other(String),
}

impl fmt::Display for ShariahRejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProhibitedBusiness => write!(f, "Prohibited core business activity"),
            Self::ExcessDebt => write!(f, "Interest-bearing debt exceeds policy threshold"),
            Self::ExcessInterestBearingCash => write!(
                f,
                "Interest-bearing cash and deposits exceed policy threshold"
            ),
            Self::ExcessImpureIncome => {
                write!(f, "Impermissible impure income exceeds policy threshold")
            }
            Self::OwnershipUnverified => write!(
                f,
                "Underlying beneficial ownership or custodial isolation unverified"
            ),
            Self::SyntheticExposure => write!(
                f,
                "Synthetic or derivative exposure without underlying spot backing"
            ),
            Self::MissingEvidence => write!(
                f,
                "Missing evidence or insufficient audited financial disclosures"
            ),
            Self::ReviewExpired => write!(f, "Periodic screening review has expired"),
            Self::Other(msg) => write!(f, "Other rejection reason: {}", msg),
        }
    }
}
