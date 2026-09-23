//! Shariah screening policy definitions and parameter configurations.
//!
//! # Decoupled Standards Architecture
//! External Shariah supervisory bodies and index providers establish varying benchmark
//! thresholds. For example:
//! - AAOIFI Standard No. 21 commonly applies a 30% ratio for interest-bearing debt to market capitalization.
//! - Other prominent boards (such as the Dow Jones Islamic Market Index / DJIM, MSCI Islamic,
//!   and FTSE Shariah) apply a 33% debt threshold or evaluate ratios against total assets.
//!
//! Rather than hardcoding 30% or 33% as a global absolute truth, [`ScreeningPolicy`] explicitly
//! binds quantitative thresholds to a designated [`ScreeningStandard`] and `policy_version`.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::types::{DenominatorMethod, ScreeningStandard};
use crate::constants::MAX_BPS;
use crate::types::BasisPoints;
use crate::validation::ValidationError;

/// Comparison mode for evaluating quantitative financial screening thresholds.
///
/// In Islamic finance governance, supervisory boards and index methodologies differ on boundary conditions:
/// - Textual definitions in certain standards (e.g. AAOIFI Standard No. 21) prescribe benchmark thresholds
///   as strictly below the boundary (`ratio < limit`).
/// - Other boards and commercial screening engines (e.g. DJIM, MSCI Islamic) treat benchmark ratios
///   inclusively (`ratio <= limit`).
///
/// By explicitly parameterizing [`ThresholdComparison`], the governing policy unambiguously specifies
/// whether boundary equality satisfies or fails screening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThresholdComparison {
    /// Strict upper bound: ratio must be strictly less than the threshold (`ratio < limit`).
    StrictLessThan,
    /// Inclusive upper bound: ratio may be up to and including the threshold (`ratio <= limit`).
    LessThanOrEqual,
}

impl ThresholdComparison {
    /// Evaluates whether a candidate ratio in basis points complies with the limit under this comparison rule.
    #[inline]
    pub fn is_compliant(&self, ratio_bps: u32, limit_bps: u32) -> bool {
        match self {
            Self::StrictLessThan => ratio_bps < limit_bps,
            Self::LessThanOrEqual => ratio_bps <= limit_bps,
        }
    }

    /// Evaluates compliance for u16 ratios (backwards compatibility helper).
    #[inline]
    pub fn is_compliant_u16(&self, ratio_bps: u16, limit_bps: u16) -> bool {
        self.is_compliant(ratio_bps as u32, limit_bps as u32)
    }
}

impl fmt::Display for ThresholdComparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StrictLessThan => write!(f, "< (StrictLessThan)"),
            Self::LessThanOrEqual => write!(f, "<= (LessThanOrEqual)"),
        }
    }
}

/// Deterministic policy configuration specifying the qualitative and quantitative
/// boundaries required for an asset to receive [`ShariahStatus::Approved`](crate::shariah::ShariahStatus).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreeningPolicy {
    /// Screening methodology standard (e.g. BoardApprovedV1, Aaoifi21, or Custom).
    pub standard: ScreeningStandard,
    /// Policy version or governance identifier (e.g. "v1.0", "aaoifi-21-2024").
    pub policy_version: String,
    /// Denominator calculation methodology mandated by this screening standard.
    pub denominator: DenominatorMethod,
    /// Maximum permissible interest-bearing debt relative to the denominator, in basis points.
    /// E.g., 3,000 bps = 30.00%, 3,300 bps = 33.00%.
    pub debt_limit_bps: BasisPoints,
    /// Maximum permissible cash and interest-bearing deposits relative to the denominator, in basis points.
    /// E.g., 3,000 bps = 30.00%, 3,300 bps = 33.00%.
    pub interest_bearing_cash_limit_bps: BasisPoints,
    /// Optional benchmark limit for Accounts Receivable + Cash relative to the denominator.
    /// Explicitly required by MSCI Islamic (<= 33.33%) and S&P Shariah (< 49.00%).
    pub receivables_cash_limit_bps: Option<BasisPoints>,
    /// Maximum permissible revenue from non-operating or impure incidental sources relative to total revenue.
    /// Standard benchmark is 500 bps = 5.00%.
    pub impure_income_limit_bps: BasisPoints,
    /// Explicit threshold comparison operator (StrictLessThan '<' vs LessThanOrEqual '<=').
    pub comparison: ThresholdComparison,
}

impl ScreeningPolicy {
    /// Constructs and validates a new [`ScreeningPolicy`].
    pub fn new(
        standard: ScreeningStandard,
        policy_version: impl Into<String>,
        denominator: DenominatorMethod,
        debt_limit_bps: BasisPoints,
        interest_bearing_cash_limit_bps: BasisPoints,
        receivables_cash_limit_bps: Option<BasisPoints>,
        impure_income_limit_bps: BasisPoints,
        comparison: ThresholdComparison,
    ) -> Result<Self, ValidationError> {
        let version = policy_version.into();
        if version.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "policy_version cannot be empty".to_string(),
            ));
        }

        if debt_limit_bps.0 > MAX_BPS {
            return Err(ValidationError::InvalidBasisPoints(debt_limit_bps.0));
        }
        if interest_bearing_cash_limit_bps.0 > MAX_BPS {
            return Err(ValidationError::InvalidBasisPoints(
                interest_bearing_cash_limit_bps.0,
            ));
        }
        if let Some(r) = receivables_cash_limit_bps {
            if r.0 > MAX_BPS {
                return Err(ValidationError::InvalidBasisPoints(r.0));
            }
        }
        if impure_income_limit_bps.0 > MAX_BPS {
            return Err(ValidationError::InvalidBasisPoints(
                impure_income_limit_bps.0,
            ));
        }

        Ok(Self {
            standard,
            policy_version: version,
            denominator,
            debt_limit_bps,
            interest_bearing_cash_limit_bps,
            receivables_cash_limit_bps,
            impure_income_limit_bps,
            comparison,
        })
    }

    /// Convenience constructor taking raw basis point values (u16) and comparison mode.
    pub fn from_raw_bps(
        standard: ScreeningStandard,
        policy_version: impl Into<String>,
        debt_limit_bps: u16,
        interest_bearing_cash_limit_bps: u16,
        impure_income_limit_bps: u16,
        comparison: ThresholdComparison,
    ) -> Result<Self, ValidationError> {
        Self::new(
            standard,
            policy_version,
            DenominatorMethod::AverageMarketCapMonths(12),
            BasisPoints::new(debt_limit_bps)?,
            BasisPoints::new(interest_bearing_cash_limit_bps)?,
            None,
            BasisPoints::new(impure_income_limit_bps)?,
            comparison,
        )
    }

    /// Default baseline policy approved by the initial supervisory board.
    ///
    /// Benchmark thresholds (inclusive upper bounds `<= limit`):
    /// - Denominator: 12-month trailing average market cap
    /// - Debt Limit: <= 3,000 bps (30.00%)
    /// - Cash / Deposits Limit: <= 3,000 bps (30.00%)
    /// - Receivables & Cash: None
    /// - Impure Income Limit: <= 500 bps (5.00%)
    pub fn board_approved_v1() -> Self {
        Self {
            standard: ScreeningStandard::BoardApprovedV1,
            policy_version: "v1.0".to_string(),
            denominator: DenominatorMethod::AverageMarketCapMonths(12),
            debt_limit_bps: BasisPoints(3_000),
            interest_bearing_cash_limit_bps: BasisPoints(3_000),
            receivables_cash_limit_bps: None,
            impure_income_limit_bps: BasisPoints(500),
            comparison: ThresholdComparison::LessThanOrEqual,
        }
    }

    /// Parameterized preset for AAOIFI Shariah Standard No. 21 with specific policy version.
    ///
    /// Standard AAOIFI textual ratios (strict upper bounds `< limit`):
    /// - Denominator: 12-month trailing average market capitalization
    /// - Debt / Market Cap < 30.00% (3,000 bps)
    /// - Cash & Deposits / Market Cap < 30.00% (3,000 bps)
    /// - Receivables & Cash: None
    /// - Impure Revenue / Total Revenue < 5.00% (500 bps)
    pub fn aaoifi_21(policy_version: impl Into<String>) -> Self {
        Self {
            standard: ScreeningStandard::Aaoifi21,
            policy_version: policy_version.into(),
            denominator: DenominatorMethod::AverageMarketCapMonths(12),
            debt_limit_bps: BasisPoints(3_000),
            interest_bearing_cash_limit_bps: BasisPoints(3_000),
            receivables_cash_limit_bps: None,
            impure_income_limit_bps: BasisPoints(500),
            comparison: ThresholdComparison::StrictLessThan,
        }
    }

    /// Default AAOIFI 21 preset for the standard 2024 review cycle.
    pub fn aaoifi_21_default() -> Self {
        Self::aaoifi_21("aaoifi-21-2024")
    }

    /// Preset for MSCI Islamic Index screening methodology.
    ///
    /// Benchmarks against total assets or market cap with explicit receivables screen:
    /// - Denominator: Total Assets
    /// - Debt / Total Assets < 33.33% (3,333 bps)
    /// - Cash & Interest-bearing Securities / Total Assets < 33.33% (3,333 bps)
    /// - Accounts Receivable + Cash / Total Assets < 33.33% (3,333 bps)
    /// - Impure Revenue / Total Revenue < 5.00% (500 bps)
    pub fn msci_islamic(policy_version: impl Into<String>) -> Self {
        Self {
            standard: ScreeningStandard::Custom("MSCI-Islamic".to_string()),
            policy_version: policy_version.into(),
            denominator: DenominatorMethod::TotalAssets,
            debt_limit_bps: BasisPoints(3_333),
            interest_bearing_cash_limit_bps: BasisPoints(3_333),
            receivables_cash_limit_bps: Some(BasisPoints(3_333)),
            impure_income_limit_bps: BasisPoints(500),
            comparison: ThresholdComparison::StrictLessThan,
        }
    }

    /// Preset for S&P Shariah Index screening methodology.
    ///
    /// Benchmarks against 36-month average market capitalization:
    /// - Denominator: 36-month average market cap
    /// - Debt / 36-mo Market Cap < 33.00% (3,300 bps)
    /// - Cash & Interest-bearing Securities / 36-mo Market Cap < 33.00% (3,300 bps)
    /// - Accounts Receivable / 36-mo Market Cap < 49.00% (4,900 bps)
    /// - Impure Revenue / Total Revenue < 5.00% (500 bps)
    pub fn sp_shariah(policy_version: impl Into<String>) -> Self {
        Self {
            standard: ScreeningStandard::Custom("SP-Shariah".to_string()),
            policy_version: policy_version.into(),
            denominator: DenominatorMethod::AverageMarketCapMonths(36),
            debt_limit_bps: BasisPoints(3_300),
            interest_bearing_cash_limit_bps: BasisPoints(3_300),
            receivables_cash_limit_bps: Some(BasisPoints(4_900)),
            impure_income_limit_bps: BasisPoints(500),
            comparison: ThresholdComparison::StrictLessThan,
        }
    }

    /// Preset for FTSE IdealRatings Shariah Index screening methodology.
    ///
    /// Benchmarks against 24-month average daily market capitalization:
    /// - Denominator: 24-month average market cap
    /// - Debt / 24-mo Market Cap < 33.33% (3,333 bps)
    /// - Cash & Interest-bearing Securities / 24-mo Market Cap < 33.33% (3,333 bps)
    /// - Accounts Receivable & Cash / 24-mo Market Cap < 50.00% (5,000 bps)
    /// - Impure Revenue / Total Revenue < 5.00% (500 bps)
    pub fn ftse_idealratings(policy_version: impl Into<String>) -> Self {
        Self {
            standard: ScreeningStandard::Custom("FTSE-IdealRatings".to_string()),
            policy_version: policy_version.into(),
            denominator: DenominatorMethod::AverageMarketCapMonths(24),
            debt_limit_bps: BasisPoints(3_333),
            interest_bearing_cash_limit_bps: BasisPoints(3_333),
            receivables_cash_limit_bps: Some(BasisPoints(5_000)),
            impure_income_limit_bps: BasisPoints(500),
            comparison: ThresholdComparison::StrictLessThan,
        }
    }

    /// Constructs a policy for a custom board standard.
    pub fn custom(
        standard_name: impl Into<String>,
        policy_version: impl Into<String>,
        debt_limit_bps: BasisPoints,
        interest_bearing_cash_limit_bps: BasisPoints,
        impure_income_limit_bps: BasisPoints,
        comparison: ThresholdComparison,
    ) -> Result<Self, ValidationError> {
        let name = standard_name.into();
        if name.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "custom standard_name cannot be empty".to_string(),
            ));
        }

        Self::new(
            ScreeningStandard::Custom(name),
            policy_version,
            DenominatorMethod::AverageMarketCapMonths(12),
            debt_limit_bps,
            interest_bearing_cash_limit_bps,
            None,
            impure_income_limit_bps,
            comparison,
        )
    }

    /// Builder method to set the denominator calculation method.
    pub fn with_denominator(mut self, denominator: DenominatorMethod) -> Self {
        self.denominator = denominator;
        self
    }

    /// Builder method to configure or remove the Accounts Receivable + Cash limit.
    pub fn with_receivables_limit(mut self, limit: Option<BasisPoints>) -> Self {
        self.receivables_cash_limit_bps = limit;
        self
    }

    /// Builder method to override the threshold comparison operator.
    pub fn with_comparison(mut self, comparison: ThresholdComparison) -> Self {
        self.comparison = comparison;
        self
    }

    /// Validates internal consistency of policy limits.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.policy_version.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "policy_version cannot be empty".to_string(),
            ));
        }
        if self.debt_limit_bps.0 > MAX_BPS {
            return Err(ValidationError::InvalidBasisPoints(self.debt_limit_bps.0));
        }
        if self.interest_bearing_cash_limit_bps.0 > MAX_BPS {
            return Err(ValidationError::InvalidBasisPoints(
                self.interest_bearing_cash_limit_bps.0,
            ));
        }
        if let Some(r) = self.receivables_cash_limit_bps {
            if r.0 > MAX_BPS {
                return Err(ValidationError::InvalidBasisPoints(r.0));
            }
        }
        if self.impure_income_limit_bps.0 > MAX_BPS {
            return Err(ValidationError::InvalidBasisPoints(
                self.impure_income_limit_bps.0,
            ));
        }
        Ok(())
    }
}

impl Default for ScreeningPolicy {
    fn default() -> Self {
        Self::board_approved_v1()
    }
}
