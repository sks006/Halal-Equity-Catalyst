use serde::{Deserialize, Serialize};
use std::fmt;

use super::ownership::OwnershipRecord;
use super::policy::ScreeningPolicy;
use super::types::{DenominatorMethod, ShariahRejectionReason};

/// Broad industry and economic sector classification.
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
    /// Evaluates intrinsic permissibility of the broad industry category.
    ///
    /// # Fail-Closed Security Model:
    /// Broad sectors like Technology, Healthcare, Manufacturing cannot be assumed compliant
    /// without qualitative activity investigation, and thus yield `RequiresReview`.
    /// Intrinsically forbidden sectors (Alcohol, Gambling, etc.) always yield `Prohibited`.
    pub fn intrinsic_classification(&self) -> BusinessClassification {
        match self {
            Self::ConventionalFinance
            | Self::Alcohol
            | Self::Gambling
            | Self::Tobacco
            | Self::Weapons
            | Self::AdultEntertainment => BusinessClassification::Prohibited,
            Self::Technology | Self::Healthcare | Self::Manufacturing | Self::Other(_) => {
                BusinessClassification::RequiresReview
            }
        }
    }

    /// Returns true if this sector is inherently prohibited in Islamic jurisprudence.
    pub fn is_inherently_prohibited(&self) -> bool {
        self.intrinsic_classification().is_prohibited()
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

/// Qualitative Shariah screening classification state.
///
/// Rather than assuming broad sectors (Technology, Healthcare, Manufacturing) are universally
/// compliant, this expresses verified screening state following qualitative review of
/// company activities, subsidiaries, and non-permissible revenue streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BusinessClassification {
    /// Vetted and approved qualitative business operations.
    Approved,
    /// Prohibited core business or excessive non-permissible activity.
    Prohibited,
    /// Unreviewed, pending investigation, or ambiguous activity (fail-closed).
    RequiresReview,
}

impl BusinessClassification {
    /// Returns true if this business activity is vetted and approved.
    pub fn is_approved(self) -> bool {
        matches!(self, Self::Approved)
    }

    /// Returns true if this business activity is prohibited.
    pub fn is_prohibited(self) -> bool {
        matches!(self, Self::Prohibited)
    }

    /// Returns true if this business activity requires review before qualification.
    pub fn requires_review(self) -> bool {
        matches!(self, Self::RequiresReview)
    }
}

impl fmt::Display for BusinessClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Approved => write!(f, "Approved"),
            Self::Prohibited => write!(f, "Prohibited"),
            Self::RequiresReview => write!(f, "RequiresReview"),
        }
    }
}

/// Qualitative business activity assessment dossier.
///
/// Ensures companies in broad sectors are investigated for prohibited revenue
/// and subsidiary activities (e.g. gambling apps, predatory lending, adult platforms,
/// interest-bearing financing arms) before being certified compliant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusinessActivityAssessment {
    /// Primary sector or industry grouping.
    pub primary_category: BusinessCategory,
    /// Detailed description of primary business operations and core revenue drivers.
    pub primary_activity_description: String,
    /// Subsidiary operations or secondary business segments.
    pub subsidiary_activities: Vec<String>,
    /// Percentage of total revenue derived from incidental prohibited/non-permissible activities, in basis points.
    pub prohibited_revenue_bps: u32,
    /// Explicit qualitative verification that business operations and subsidiaries have been vetted.
    pub activities_reviewed: bool,
    /// Verifiable audit or evidence source (e.g. "SEC Form 10-K FY2026", "Audited Annual Report", "BICS Classification").
    pub assessment_source: String,
}

impl BusinessActivityAssessment {
    /// Creates a verified qualitative assessment for an investigated business.
    pub fn reviewed_permissible(
        primary_category: BusinessCategory,
        primary_activity_description: impl Into<String>,
        assessment_source: impl Into<String>,
    ) -> Self {
        Self {
            primary_category,
            primary_activity_description: primary_activity_description.into(),
            subsidiary_activities: Vec::new(),
            prohibited_revenue_bps: 0,
            activities_reviewed: true,
            assessment_source: assessment_source.into(),
        }
    }

    /// Convenience constructor creating an unreviewed assessment that requires qualitative investigation.
    pub fn unreviewed(primary_category: BusinessCategory) -> Self {
        Self {
            primary_category,
            primary_activity_description: String::new(),
            subsidiary_activities: Vec::new(),
            prohibited_revenue_bps: 0,
            activities_reviewed: false,
            assessment_source: "Pending qualitative review".to_string(),
        }
    }

    /// Creates an assessment for a business confirmed to engage in prohibited activities.
    pub fn prohibited(
        primary_category: BusinessCategory,
        primary_activity_description: impl Into<String>,
    ) -> Self {
        Self {
            primary_category,
            primary_activity_description: primary_activity_description.into(),
            subsidiary_activities: Vec::new(),
            prohibited_revenue_bps: 10_000,
            activities_reviewed: true,
            assessment_source: "Negative qualitative screen".to_string(),
        }
    }

    /// Builder method to attach subsidiary activities.
    pub fn with_subsidiary(mut self, activity: impl Into<String>) -> Self {
        self.subsidiary_activities.push(activity.into());
        self
    }

    /// Builder method to record non-operating prohibited revenue percentage in basis points.
    pub fn with_prohibited_revenue_bps(mut self, bps: u32) -> Self {
        self.prohibited_revenue_bps = bps;
        self
    }

    /// Builder method to set the review verification flag.
    pub fn with_reviewed(mut self, reviewed: bool) -> Self {
        self.activities_reviewed = reviewed;
        self
    }

    /// Evaluates the qualitative [`BusinessClassification`].
    pub fn classification(&self) -> BusinessClassification {
        // 1. Intrinsically prohibited sectors always fail immediately
        if self.primary_category.is_inherently_prohibited() {
            return BusinessClassification::Prohibited;
        }

        // 2. Unreviewed activities cannot pass screening (fail-closed)
        if !self.activities_reviewed {
            return BusinessClassification::RequiresReview;
        }

        // 3. Excessive prohibited revenue (> 5.00% benchmark) disqualifies the business
        if self.prohibited_revenue_bps > 500 {
            return BusinessClassification::Prohibited;
        }

        // 4. Other/unspecified category requires non-empty description
        if matches!(self.primary_category, BusinessCategory::Other(_))
            && self.primary_activity_description.trim().is_empty()
        {
            return BusinessClassification::RequiresReview;
        }

        BusinessClassification::Approved
    }
}

/// Normalized financial screening metrics evaluated against a [`ScreeningPolicy`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShariahFinancialMetrics {
    /// Trailing total interest-bearing debt relative to the policy denominator, in basis points.
    pub debt_ratio_bps: u32,
    /// Trailing cash and interest-bearing deposits relative to the policy denominator, in basis points.
    pub interest_bearing_cash_ratio_bps: u32,
    /// Accounts receivable and cash relative to the policy denominator, in basis points.
    /// Evaluated under MSCI Islamic (<= 33.33%) and S&P Shariah (< 49.00%) policies.
    pub receivables_cash_ratio_bps: Option<u32>,
    /// Incidental or non-operating impermissible revenue relative to total revenue, in basis points.
    pub impure_income_ratio_bps: u32,
    /// Calculation methodology utilized for the denominator.
    pub denominator_method: DenominatorMethod,
}

impl ShariahFinancialMetrics {
    /// Creates a complete financial metrics record with explicit denominator method and receivables ratio.
    pub const fn new(
        debt_ratio_bps: u32,
        interest_bearing_cash_ratio_bps: u32,
        receivables_cash_ratio_bps: Option<u32>,
        impure_income_ratio_bps: u32,
        denominator_method: DenominatorMethod,
    ) -> Self {
        Self {
            debt_ratio_bps,
            interest_bearing_cash_ratio_bps,
            receivables_cash_ratio_bps,
            impure_income_ratio_bps,
            denominator_method,
        }
    }

    /// Convenience constructor from market-cap based ratios (12-month average market cap).
    pub const fn from_market_cap_ratios(
        debt_ratio_bps: u32,
        interest_bearing_cash_ratio_bps: u32,
        impure_income_ratio_bps: u32,
    ) -> Self {
        Self {
            debt_ratio_bps,
            interest_bearing_cash_ratio_bps,
            receivables_cash_ratio_bps: None,
            impure_income_ratio_bps,
            denominator_method: DenominatorMethod::AverageMarketCapMonths(12),
        }
    }

    /// Builder method to attach accounts receivable + cash ratio.
    pub fn with_receivables_ratio(mut self, receivables_bps: u32) -> Self {
        self.receivables_cash_ratio_bps = Some(receivables_bps);
        self
    }
}

// --- Re-exports from dedicated purification module (Phase 15) ---
pub use super::purification::{
    assess_direct_impure_value, assess_dividend_purification,
    assess_per_share_dividend_purification, assess_purification,
    assess_purification_for_registered_asset, calculate_purification,
    calculate_purification_value_minor_units, CurrencyCode, DirectValuePurificationRequest,
    DividendPurificationRequest, PerShareDividendPurificationRequest, PurificationAssessment,
    PurificationError, PurificationRoundingRule,
};

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

/// Evaluates qualitative business operations under a strict qualitative investigation model.
pub fn screen_business_activity(
    assessment: &BusinessActivityAssessment,
) -> Result<(), ShariahRejectionReason> {
    match assessment.classification() {
        BusinessClassification::Approved => Ok(()),
        BusinessClassification::Prohibited => Err(ShariahRejectionReason::ProhibitedBusiness),
        BusinessClassification::RequiresReview => {
            Err(ShariahRejectionReason::BusinessClassificationRequiresReview)
        }
    }
}

/// Standalone check evaluating business activity classification. Alias for [`screen_business_activity`].
pub fn check_business_activity(
    assessment: &BusinessActivityAssessment,
) -> Result<(), ShariahRejectionReason> {
    screen_business_activity(assessment)
}

/// Evaluates quantitative capital structure, cash, liquidity, and revenue purity ratios against a [`ScreeningPolicy`].
pub fn screen_financial_metrics(
    metrics: &ShariahFinancialMetrics,
    policy: &ScreeningPolicy,
) -> Result<(), ShariahRejectionReason> {
    // 1. Debt ratio benchmark
    if !policy.comparison.is_compliant(
        metrics.debt_ratio_bps,
        policy.debt_limit_bps.as_bps() as u32,
    ) {
        return Err(ShariahRejectionReason::ExcessDebt);
    }

    // 2. Interest-bearing cash ratio benchmark
    if !policy.comparison.is_compliant(
        metrics.interest_bearing_cash_ratio_bps,
        policy.interest_bearing_cash_limit_bps.as_bps() as u32,
    ) {
        return Err(ShariahRejectionReason::ExcessInterestBearingCash);
    }

    // 3. Accounts Receivable + Cash screen (when mandated by methodology, e.g. MSCI / S&P)
    if let Some(limit) = policy.receivables_cash_limit_bps {
        match metrics.receivables_cash_ratio_bps {
            Some(ratio) => {
                if !policy.comparison.is_compliant(ratio, limit.as_bps() as u32) {
                    return Err(ShariahRejectionReason::ExcessReceivablesAndCash);
                }
            }
            None => return Err(ShariahRejectionReason::MissingEvidence),
        }
    }

    // 4. Impure income revenue purity benchmark
    if !policy.comparison.is_compliant(
        metrics.impure_income_ratio_bps,
        policy.impure_income_limit_bps.as_bps() as u32,
    ) {
        return Err(ShariahRejectionReason::ExcessImpureIncome);
    }

    Ok(())
}

/// Fully deterministic asset screening engine evaluating ownership, qualitative business activities,
/// and quantitative metrics against the active [`ScreeningPolicy`].
pub fn screen_asset(
    business: &BusinessActivityAssessment,
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

    // 2. Qualitative business activity investigation
    if let Err(reason) = screen_business_activity(business) {
        return ScreeningResult::Rejected { reason };
    }

    // 3. Quantitative financial benchmarks
    if let Err(reason) = screen_financial_metrics(metrics, policy) {
        return ScreeningResult::Rejected { reason };
    }

    ScreeningResult::Approved
}
