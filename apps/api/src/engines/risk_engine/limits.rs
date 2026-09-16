//! Policy limits and operational boundary checks.

use equity_catalyst_shared::allocation::RebalanceTrade;

/// Maximum allowable slippage for any rebalancing trade (3.00% = 300 bps)
pub const MAX_PERMISSIBLE_SLIPPAGE_BPS: u16 = 300;

/// Maximum allowable single trade size relative to total portfolio (10.00% = 1000 bps)
pub const MAX_SINGLE_TRADE_BPS: u16 = 1000;

/// Validates that proposed trades satisfy general size and slippage constraints.
pub fn validate_trade_limits(
    trades: &[RebalanceTrade],
    total_portfolio_usd: u64,
) -> Result<(), String> {
    for trade in trades {
        if trade.trade_value == 0 {
            return Err(format!("Trade for {} has zero USD value", trade.symbol));
        }

        if let Some(trade_bps) = (trade.trade_value * 10_000).checked_div(total_portfolio_usd) {
            if trade_bps > MAX_SINGLE_TRADE_BPS as u64 {
                return Err(format!(
                    "Trade size ${} for {} exceeds single trade limit of 10.00% ({} bps > {} bps)",
                    trade.trade_value, trade.symbol, trade_bps, MAX_SINGLE_TRADE_BPS
                ));
            }
        }

        if trade.trade_value > total_portfolio_usd {
            return Err(format!(
                "Trade size ${} for {} exceeds total portfolio value ${}",
                trade.trade_value, trade.symbol, total_portfolio_usd
            ));
        }
    }

    Ok(())
}
