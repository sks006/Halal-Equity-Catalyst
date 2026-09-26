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

/// Denominator methodology utilized for evaluating leverage, cash, and liquidity screening ratios.
///
/// Distinct index and supervisory bodies mandate specific denominator standards:
/// - S&P Shariah uses a 36-month average market value.
/// - FTSE IdealRatings specifies a 24-month average daily market cap (with a total-assets fallback).
/// - AAOIFI Standard 21 utilizes 12-month average market cap or total assets depending on asset class.
/// - MSCI Islamic utilizes total assets or average market capitalization depending on index series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DenominatorMethod {
    /// Spot market capitalization as of the balance sheet or screening date.
    CurrentMarketCap,
    /// Trailing average daily market capitalization over N months (e.g. 12, 24, or 36).
    AverageMarketCapMonths(u8),
    /// Total assets reported on the audited balance sheet.
    TotalAssets,
}

impl fmt::Display for DenominatorMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentMarketCap => write!(f, "CurrentMarketCap"),
            Self::AverageMarketCapMonths(m) => write!(f, "AverageMarketCap({} months)", m),
            Self::TotalAssets => write!(f, "TotalAssets"),
        }
    }
}

/// Specific qualitative, quantitative, or structural reason for Shariah screening rejection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShariahRejectionReason {
    /// Core business activity violates Islamic principles (conventional banking, alcohol, gambling,
    /// tobacco, adult entertainment, weapons, etc.).
    ProhibitedBusiness,
    /// Total interest-bearing debt exceeds the policy limit relative to denominator.
    ExcessDebt,
    /// Total cash and interest-bearing deposits exceed the policy limit relative to denominator.
    ExcessInterestBearingCash,
    /// Accounts receivable and cash exceed the policy limit relative to denominator (e.g. MSCI Islamic test).
    ExcessReceivablesAndCash,
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
    /// Core business activity classification is unknown, unclassified, or unverified (fail-closed).
    BusinessClassificationUnknown,
    /// Business activities require further qualitative investigation or supervisory board audit.
    BusinessClassificationRequiresReview,
    /// Denominator calculation method does not match governing screening policy requirements.
    DenominatorMethodMismatch,
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
            Self::ExcessReceivablesAndCash => {
                write!(f, "Accounts receivable and cash exceed policy threshold")
            }
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
            Self::BusinessClassificationUnknown => {
                write!(
                    f,
                    "Business activity classification is unknown or unverified"
                )
            }
            Self::BusinessClassificationRequiresReview => {
                write!(
                    f,
                    "Business activity requires qualitative investigation or audit"
                )
            }
            Self::DenominatorMethodMismatch => {
                write!(
                    f,
                    "Financial metrics denominator method does not match policy requirements"
                )
            }
            Self::Other(msg) => write!(f, "Other rejection reason: {}", msg),
        }
    }
}
