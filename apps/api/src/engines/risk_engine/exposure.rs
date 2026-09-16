//! Single-asset portfolio concentration exposure validation.

use equity_catalyst_shared::{risk::check_position_exposure, types::BasisPoints};

/// Validates that an asset's projected post-trade value does not violate max position exposure.
pub fn validate_position_exposure(
    post_trade_value_usd: u64,
    total_portfolio_usd: u64,
    max_position_bps: u16,
) -> Result<(), String> {
    if total_portfolio_usd == 0 {
        return Ok(());
    }

    check_position_exposure(
        post_trade_value_usd,
        total_portfolio_usd,
        BasisPoints(max_position_bps),
    )
    .map_err(|e| format!("Exposure violation: {}", e))
}
