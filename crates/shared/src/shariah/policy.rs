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

use crate::constants::MAX_BPS;
use crate::types::BasisPoints;
use crate::validation::ValidationError;
use super::types::ScreeningStandard;

/// Deterministic policy configuration specifying the qualitative and quantitative
/// boundaries required for an asset to receive [`ShariahStatus::Approved`](crate::shariah::ShariahStatus).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreeningPolicy {
    /// Screening methodology standard (e.g. BoardApprovedV1, Aaoifi21, or Custom).
    pub standard: ScreeningStandard,
    /// Policy version or governance identifier (e.g. "v1.0", "aaoifi-21-2024").
    pub policy_version: String,
    /// Maximum permissible interest-bearing debt relative to market capitalization, in basis points.
    /// E.g., 3,000 bps = 30.00%, 3,300 bps = 33.00%.
    pub debt_limit_bps: BasisPoints,
    /// Maximum permissible cash and interest-bearing deposits relative to market capitalization, in basis points.
    /// E.g., 3,000 bps = 30.00%, 3,300 bps = 33.00%.
    pub interest_bearing_cash_limit_bps: BasisPoints,
    /// Maximum permissible revenue from non-operating or impure incidental sources relative to total revenue.
    /// Standard benchmark is 500 bps = 5.00%.
    pub impure_income_limit_bps: BasisPoints,
}

impl ScreeningPolicy {
    /// Constructs and validates a new [`ScreeningPolicy`].
    pub fn new(
        standard: ScreeningStandard,
        policy_version: impl Into<String>,
        debt_limit_bps: BasisPoints,
        interest_bearing_cash_limit_bps: BasisPoints,
        impure_income_limit_bps: BasisPoints,
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
        if impure_income_limit_bps.0 > MAX_BPS {
            return Err(ValidationError::InvalidBasisPoints(
                impure_income_limit_bps.0,
            ));
        }

        Ok(Self {
            standard,
            policy_version: version,
            debt_limit_bps,
            interest_bearing_cash_limit_bps,
            impure_income_limit_bps,
        })
    }

    /// Convenience constructor taking raw basis point values (u16).
    pub fn from_raw_bps(
        standard: ScreeningStandard,
        policy_version: impl Into<String>,
        debt_limit_bps: u16,
        interest_bearing_cash_limit_bps: u16,
        impure_income_limit_bps: u16,
    ) -> Result<Self, ValidationError> {
        Self::new(
            standard,
            policy_version,
            BasisPoints::new(debt_limit_bps)?,
            BasisPoints::new(interest_bearing_cash_limit_bps)?,
            BasisPoints::new(impure_income_limit_bps)?,
        )
    }

    /// Default baseline policy approved by the initial supervisory board.
    ///
    /// Benchmark thresholds:
    /// - Debt Limit: 3,000 bps (30.00%)
    /// - Cash / Deposits Limit: 3,000 bps (30.00%)
    /// - Impure Income Limit: 500 bps (5.00%)
    pub fn board_approved_v1() -> Self {
        Self {
            standard: ScreeningStandard::BoardApprovedV1,
            policy_version: "v1.0".to_string(),
            debt_limit_bps: BasisPoints(3_000),
            interest_bearing_cash_limit_bps: BasisPoints(3_000),
            impure_income_limit_bps: BasisPoints(500),
        }
    }

    /// Parameterized preset for AAOIFI Shariah Standard No. 21 with specific policy version.
    ///
    /// Standard AAOIFI ratios:
    /// - Debt / Market Cap < 30.00% (3,000 bps)
    /// - Cash & Deposits / Market Cap < 30.00% (3,000 bps)
    /// - Impure Revenue / Total Revenue < 5.00% (500 bps)
    pub fn aaoifi_21(policy_version: impl Into<String>) -> Self {
        Self {
            standard: ScreeningStandard::Aaoifi21,
            policy_version: policy_version.into(),
            debt_limit_bps: BasisPoints(3_000),
            interest_bearing_cash_limit_bps: BasisPoints(3_000),
            impure_income_limit_bps: BasisPoints(500),
        }
    }

    /// Default AAOIFI 21 preset for the standard 2024 review cycle.
    pub fn aaoifi_21_default() -> Self {
        Self::aaoifi_21("aaoifi-21-2024")
    }

    /// Constructs a policy for a custom board standard (e.g. DJIM 33% debt threshold).
    pub fn custom(
        standard_name: impl Into<String>,
        policy_version: impl Into<String>,
        debt_limit_bps: BasisPoints,
        interest_bearing_cash_limit_bps: BasisPoints,
        impure_income_limit_bps: BasisPoints,
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
            debt_limit_bps,
            interest_bearing_cash_limit_bps,
            impure_income_limit_bps,
        )
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
