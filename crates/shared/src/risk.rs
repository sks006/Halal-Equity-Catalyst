//! Pure deterministic risk engine calculations.

use serde::{Deserialize, Serialize};

use crate::math::calculate_basis_points;
use crate::types::BasisPoints;
use crate::validation::ValidationError;

/// Result of evaluating an action against risk policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub is_approved: bool,
    pub rejection_reason: Option<String>,
    pub evaluated_ltv: BasisPoints,
    pub evaluated_exposure: BasisPoints,
}

/// Calculate Loan-to-Value (LTV) ratio in basis points.
/// ltv = (loan_amount * 10,000) / collateral_value
pub fn calculate_ltv(loan_amount: u64, collateral_value: u64) -> Result<BasisPoints, ValidationError> {
    if collateral_value == 0 {
        if loan_amount > 0 {
            return Ok(BasisPoints::MAX);
        }
        return Ok(BasisPoints::ZERO);
    }
    calculate_basis_points(loan_amount, collateral_value)
}

/// Verify that current LTV does not breach maximum LTV limit.
pub fn check_ltv_limit(ltv: BasisPoints, max_ltv: BasisPoints) -> Result<(), ValidationError> {
    if ltv > max_ltv {
        return Err(ValidationError::ExceedsMaxLtv {
            actual: ltv.0,
            limit: max_ltv.0,
        });
    }
    Ok(())
}

/// Calculate position exposure as a percentage of total portfolio value.
pub fn calculate_exposure(
    position_value: u64,
    total_portfolio_value: u64,
) -> Result<BasisPoints, ValidationError> {
    calculate_basis_points(position_value, total_portfolio_value)
}

/// Verify that a single position does not exceed the allowed portfolio exposure threshold.
pub fn check_position_exposure(
    position_value: u64,
    total_portfolio_value: u64,
    max_position_bps: BasisPoints,
) -> Result<(), ValidationError> {
    let exposure = calculate_exposure(position_value, total_portfolio_value)?;
    if exposure > max_position_bps {
        return Err(ValidationError::ExceedsMaxPosition {
            actual: exposure.0,
            limit: max_position_bps.0,
        });
    }
    Ok(())
}

/// Check if position loss relative to entry price exceeds stop-loss threshold.
pub fn is_stop_loss_triggered(
    entry_price: u64,
    current_price: u64,
    stop_loss_bps: BasisPoints,
) -> bool {
    if entry_price == 0 || current_price >= entry_price {
        return false;
    }
    let loss = entry_price - current_price;
    if let Ok(loss_bps) = calculate_basis_points(loss, entry_price) {
        loss_bps >= stop_loss_bps
    } else {
        false
    }
}

/// Check if position gain relative to entry price exceeds take-profit threshold.
pub fn is_take_profit_triggered(
    entry_price: u64,
    current_price: u64,
    take_profit_bps: BasisPoints,
) -> bool {
    if entry_price == 0 || current_price <= entry_price {
        return false;
    }
    let gain = current_price - entry_price;
    if let Ok(gain_bps) = calculate_basis_points(gain, entry_price) {
        gain_bps >= take_profit_bps
    } else {
        false
    }
}
