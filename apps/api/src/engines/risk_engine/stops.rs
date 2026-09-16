//! Stop-loss and take-profit condition checks.

use equity_catalyst_shared::{
    allocation::RebalanceTrade, risk::is_stop_loss_triggered, types::BasisPoints,
};

use crate::models::PortfolioModel;

/// Validates that proposed trades do not buy further into an asset that has triggered stop-loss.
pub fn validate_stop_conditions(
    trades: &[RebalanceTrade],
    positions: &[PortfolioModel],
    stop_loss_bps: u16,
) -> Result<(), String> {
    for trade in trades {
        if trade.is_buy {
            if let Some(pos) = positions.iter().find(|p| p.asset_symbol == trade.symbol) {
                let entry = (pos.entry_price_usd * 100.0) as u64;
                let current = (pos.current_price_usd * 100.0) as u64;

                if entry > 0 && is_stop_loss_triggered(entry, current, BasisPoints(stop_loss_bps)) {
                    return Err(format!(
                        "Stop-loss active on {} (entry: ${:.2}, current: ${:.2}); cannot execute additional Buy",
                        trade.symbol, pos.entry_price_usd, pos.current_price_usd
                    ));
                }
            }
        }
    }

    Ok(())
}
