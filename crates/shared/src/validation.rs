//! Domain validation primitives for Equity Catalyst.

use thiserror::Error;
use std::collections::HashSet;

use crate::constants::MAX_BPS;
use crate::types::{AssetWeight, BasisPoints, RiskLimits};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Basis points value {0} exceeds maximum allowed 10,000 (100.00%)")]
    InvalidBasisPoints(u16),

    #[error("Portfolio weights sum to {actual} bps, but must sum to {expected} bps (100.00%)")]
    WeightsDoNotSumTo100Percent { actual: u16, expected: u16 },

    #[error("Loan-to-value ratio {actual} bps exceeds maximum policy limit {limit} bps")]
    ExceedsMaxLtv { actual: u16, limit: u16 },

    #[error("Position exposure {actual} bps exceeds maximum policy limit {limit} bps")]
    ExceedsMaxPosition { actual: u16, limit: u16 },

    #[error("Slippage {actual} bps exceeds maximum limit {limit} bps")]
    ExceedsMaxSlippage { actual: u16, limit: u16 },

    #[error("Allocation target cannot have empty weights")]
    EmptyAllocationWeights,

    #[error("Duplicate asset symbol in weights: {0}")]
    DuplicateAssetSymbol(String),

    #[error("Calculation resulted in arithmetic overflow")]
    ArithmeticOverflow,

    #[error("Zero total portfolio value cannot be rebalanced")]
    ZeroPortfolioValue,
}

pub fn validate_bps(bps: u16) -> Result<BasisPoints, ValidationError> {
    if bps > MAX_BPS {
        return Err(ValidationError::InvalidBasisPoints(bps));
    }
    Ok(BasisPoints(bps))
}

pub fn validate_weights_sum(weights: &[AssetWeight]) -> Result<(), ValidationError> {
    if weights.is_empty() {
        return Err(ValidationError::EmptyAllocationWeights);
    }

    let mut seen = HashSet::new();
    let mut total: u16 = 0;

    for w in weights {
        if !seen.insert(&w.symbol) {
            return Err(ValidationError::DuplicateAssetSymbol(w.symbol.clone()));
        }
        total = total
            .checked_add(w.target_weight.0)
            .ok_or(ValidationError::ArithmeticOverflow)?;
    }

    if total != MAX_BPS {
        return Err(ValidationError::WeightsDoNotSumTo100Percent {
            actual: total,
            expected: MAX_BPS,
        });
    }

    Ok(())
}

pub fn validate_risk_limits(limits: &RiskLimits) -> Result<(), ValidationError> {
    validate_bps(limits.max_ltv_bps.0)?;
    validate_bps(limits.max_position_bps.0)?;
    validate_bps(limits.max_slippage_bps.0)?;
    Ok(())
}
