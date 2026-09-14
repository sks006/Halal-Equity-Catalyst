//! Loan-To-Value (LTV) boundary checks.

use equity_catalyst_shared::{
    risk::{calculate_ltv, check_ltv_limit},
    types::BasisPoints,
};

/// Validates that borrowing does not breach the policy's max LTV ratio.
pub fn validate_ltv(
    total_debt_usd: u64,
    total_collateral_usd: u64,
    max_ltv_bps: u16,
) -> Result<(), String> {
    if total_debt_usd == 0 {
        return Ok(());
    }

    if total_collateral_usd == 0 {
        return Err("Debt exists with zero collateral".to_string());
    }

    let ltv = calculate_ltv(total_debt_usd, total_collateral_usd)
        .map_err(|e| format!("LTV calculation error: {}", e))?;

    check_ltv_limit(ltv, BasisPoints(max_ltv_bps))
        .map_err(|e| format!("LTV limit violation: {}", e))
}
