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
    pub evaluated_cash_bps: BasisPoints,
    pub evaluated_exposure: BasisPoints,
}

/// Calculate unencumbered cash reserve ratio in basis points.
/// cash_bps = (cash_amount * 10,000) / total_portfolio_value
pub fn calculate_cash_reserve(
    cash_amount: u64,
    total_portfolio_value: u64,
) -> Result<BasisPoints, ValidationError> {
    if total_portfolio_value == 0 {
        return Ok(BasisPoints::ZERO);
    }
    calculate_basis_points(cash_amount, total_portfolio_value)
}

/// Verify that available cash reserve meets or exceeds the required minimum policy threshold.
pub fn check_cash_reserve(
    cash_bps: BasisPoints,
    min_cash_bps: BasisPoints,
) -> Result<(), ValidationError> {
    if cash_bps < min_cash_bps {
        return Err(ValidationError::BelowMinCashReserve {
            actual: cash_bps.0,
            limit: min_cash_bps.0,
        });
    }
    Ok(())
}

/// Enforce that only spot trading is permitted (no short selling, margin borrowing, or leverage).
pub fn enforce_spot_only(is_short: bool, leverage_multiple: f64) -> Result<(), ValidationError> {
    if is_short || leverage_multiple > 1.0 {
        return Err(ValidationError::ProhibitedLeverageOrShort);
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

/// Enforces deterministic spot ownership before sale (*Bay' ma la Yamlik* prohibition).
///
/// A vault or trader is strictly forbidden from selling shares they do not beneficially possess.
///
/// # Invariants
/// - Rejects sells where `requested_quantity == 0` (non-positive quantity)
/// - Rejects sells where `available_quantity == 0` (zero-balance naked sale)
/// - Rejects sells where `requested_quantity > available_quantity` (oversell / partial naked sale)
pub fn validate_spot_ownership(
    symbol: &str,
    requested_quantity: u64,
    available_quantity: u64,
) -> Result<(), ValidationError> {
    if symbol.trim().is_empty() {
        return Err(ValidationError::EmptySymbol);
    }

    if requested_quantity == 0 {
        return Err(ValidationError::InvalidParam(
            "Requested sell quantity must be positive and non-zero".to_string(),
        ));
    }

    if available_quantity == 0 {
        return Err(ValidationError::ProhibitedLeverageOrShort);
    }

    if requested_quantity > available_quantity {
        return Err(ValidationError::ProhibitedLeverageOrShort);
    }

    Ok(())
}

/// Enforces 100% equity-capitalized spot funding without leverage, margin, or borrowed debt.
///
/// # Invariants
/// - Rejects if `leverage_multiple > 1.0` or `< 0.0` or `NaN` (margin leverage strictly prohibited)
/// - Rejects if `borrowed_amount_usd > 0` (interest-bearing debt strictly prohibited)
/// - Rejects if `available_settled_cash_usd < required_cash_usd` (insufficient cash reserves)
pub fn validate_spot_funding(
    required_cash_usd: u64,
    available_settled_cash_usd: u64,
    leverage_multiple: f64,
    borrowed_amount_usd: u64,
) -> Result<(), ValidationError> {
    if leverage_multiple.is_nan()
        || leverage_multiple > 1.0
        || leverage_multiple < 0.0
        || borrowed_amount_usd > 0
    {
        return Err(ValidationError::ProhibitedLeverageOrShort);
    }

    if available_settled_cash_usd < required_cash_usd {
        return Err(ValidationError::InvalidParam(format!(
            "Insufficient settled cash: required ${}, available ${}",
            required_cash_usd, available_settled_cash_usd
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_spot_ownership_rules() {
        // Valid sell: exact balance
        assert!(validate_spot_ownership("NVDA", 100, 100).is_ok());

        // Valid sell: partial balance
        assert!(validate_spot_ownership("NVDA", 50, 100).is_ok());

        // Reject zero-balance sell (naked short sale)
        let zero_bal_err = validate_spot_ownership("NVDA", 10, 0);
        assert_eq!(
            zero_bal_err,
            Err(ValidationError::ProhibitedLeverageOrShort)
        );

        // Reject partial sell exceeding available balance (oversell)
        let oversell_err = validate_spot_ownership("NVDA", 150, 100);
        assert_eq!(
            oversell_err,
            Err(ValidationError::ProhibitedLeverageOrShort)
        );

        // Reject zero quantity
        let zero_qty_err = validate_spot_ownership("NVDA", 0, 100);
        assert!(matches!(
            zero_qty_err,
            Err(ValidationError::InvalidParam(_))
        ));

        // Reject empty symbol
        let empty_sym_err = validate_spot_ownership("", 50, 100);
        assert_eq!(empty_sym_err, Err(ValidationError::EmptySymbol));
    }

    #[test]
    fn test_validate_spot_funding_rules() {
        // Valid spot buy: 100% cash funded, 1.0x leverage, 0 borrowed
        assert!(validate_spot_funding(5_000, 10_000, 1.0, 0).is_ok());
        assert!(validate_spot_funding(10_000, 10_000, 1.0, 0).is_ok());

        // Reject leverage > 1.0 (margin loan)
        let leverage_err = validate_spot_funding(5_000, 10_000, 1.5, 0);
        assert_eq!(
            leverage_err,
            Err(ValidationError::ProhibitedLeverageOrShort)
        );

        // Reject negative leverage or NaN
        assert_eq!(
            validate_spot_funding(5_000, 10_000, -0.5, 0),
            Err(ValidationError::ProhibitedLeverageOrShort)
        );
        assert_eq!(
            validate_spot_funding(5_000, 10_000, f64::NAN, 0),
            Err(ValidationError::ProhibitedLeverageOrShort)
        );

        // Reject borrowed funds > 0 (debt-financed buy)
        let borrowed_err = validate_spot_funding(5_000, 10_000, 1.0, 1_000);
        assert_eq!(
            borrowed_err,
            Err(ValidationError::ProhibitedLeverageOrShort)
        );

        // Reject insufficient settled cash
        let cash_err = validate_spot_funding(15_000, 10_000, 1.0, 0);
        assert!(matches!(cash_err, Err(ValidationError::InvalidParam(_))));
    }
}
