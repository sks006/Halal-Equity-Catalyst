//! Deterministic math utilities for Equity Catalyst.

use crate::constants::MAX_BPS;
use crate::types::BasisPoints;
use crate::validation::ValidationError;

/// Compute fractional amount corresponding to a given basis points weight.
/// result = (amount * bps) / 10,000
pub fn compute_bps_amount(amount: u64, bps: BasisPoints) -> Result<u64, ValidationError> {
    if !bps.is_valid() {
        return Err(ValidationError::InvalidBasisPoints(bps.0));
    }
    let res = (amount as u128)
        .checked_mul(bps.0 as u128)
        .ok_or(ValidationError::ArithmeticOverflow)?
        .checked_div(MAX_BPS as u128)
        .ok_or(ValidationError::ArithmeticOverflow)?;
    u64::try_from(res).map_err(|_| ValidationError::ArithmeticOverflow)
}

/// Compute the percentage representation in basis points of `part` relative to `total`.
/// bps = (part * 10,000) / total
pub fn calculate_basis_points(part: u64, total: u64) -> Result<BasisPoints, ValidationError> {
    if total == 0 {
        return Ok(BasisPoints::ZERO);
    }
    let res = (part as u128)
        .checked_mul(MAX_BPS as u128)
        .ok_or(ValidationError::ArithmeticOverflow)?
        .checked_div(total as u128)
        .ok_or(ValidationError::ArithmeticOverflow)?;

    let capped = std::cmp::min(res, MAX_BPS as u128) as u16;
    Ok(BasisPoints(capped))
}

/// Calculate absolute drift difference between two basis points values.
pub fn calculate_drift(current_bps: BasisPoints, target_bps: BasisPoints) -> BasisPoints {
    let diff = (current_bps.0 as i32 - target_bps.0 as i32).abs() as u16;
    BasisPoints(diff)
}

/// Calculate proportional shares to mint on deposit.
pub fn calculate_shares_to_mint(
    amount: u64,
    total_deposits: u64,
    total_shares: u64,
) -> Result<u64, ValidationError> {
    if total_shares == 0 || total_deposits == 0 {
        return Ok(amount);
    }
    let res = (amount as u128)
        .checked_mul(total_shares as u128)
        .ok_or(ValidationError::ArithmeticOverflow)?
        .checked_div(total_deposits as u128)
        .ok_or(ValidationError::ArithmeticOverflow)?;
    u64::try_from(res).map_err(|_| ValidationError::ArithmeticOverflow)
}

/// Calculate proportional assets to return on share redemption.
pub fn calculate_assets_to_withdraw(
    shares: u64,
    total_deposits: u64,
    total_shares: u64,
) -> Result<u64, ValidationError> {
    if total_shares == 0 {
        return Ok(0);
    }
    let res = (shares as u128)
        .checked_mul(total_deposits as u128)
        .ok_or(ValidationError::ArithmeticOverflow)?
        .checked_div(total_shares as u128)
        .ok_or(ValidationError::ArithmeticOverflow)?;
    u64::try_from(res).map_err(|_| ValidationError::ArithmeticOverflow)
}

/// Calculate slippage or price impact in basis points.
pub fn calculate_slippage_bps(
    expected_amount: u64,
    actual_amount: u64,
) -> Result<BasisPoints, ValidationError> {
    if expected_amount == 0 || actual_amount >= expected_amount {
        return Ok(BasisPoints::ZERO);
    }
    let diff = expected_amount - actual_amount;
    calculate_basis_points(diff, expected_amount)
}
