//! Risk Engine acting as the second line of defense before transaction submission.

pub mod exposure;
pub mod limits;
pub mod stops;

pub use exposure::validate_position_exposure;
pub use limits::validate_trade_limits;
pub use stops::validate_stop_conditions;

use equity_catalyst_shared::allocation::RebalanceTrade;

use crate::models::{PolicyModel, PortfolioModel};

#[derive(Debug, Clone, PartialEq)]
pub enum RiskAssessment {
    Approved,
    Rejected { reason: String },
}

#[derive(Debug, Clone, Default)]
pub struct RiskEngine;

impl RiskEngine {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates proposed trades through the complete spot risk pipeline:
    /// Proposed Action -> Position Exposure -> Policy Limits -> Cash Reserve Adequacy -> Stop Conditions -> Approve / Reject
    pub fn evaluate_proposed_trades(
        &self,
        trades: &[RebalanceTrade],
        positions: &[PortfolioModel],
        policy: &PolicyModel,
        total_portfolio_usd: u64,
        available_cash_usd: u64,
    ) -> RiskAssessment {
        if trades.is_empty() {
            return RiskAssessment::Approved;
        }

        // 1. Position exposure check: evaluate projected post-trade exposure
        for trade in trades {
            if trade.is_buy {
                let current_val = positions
                    .iter()
                    .find(|p| p.asset_symbol == trade.symbol)
                    .map(|p| p.current_value_usd as u64)
                    .unwrap_or(0);

                let post_trade_val = current_val + trade.trade_value;
                if let Err(reason) = validate_position_exposure(
                    post_trade_val,
                    total_portfolio_usd,
                    policy.max_position_bps as u16,
                ) {
                    return RiskAssessment::Rejected { reason };
                }
            }
        }

        // 2. Policy limits check
        if let Err(reason) = validate_trade_limits(trades, total_portfolio_usd) {
            return RiskAssessment::Rejected { reason };
        }

        // 3. Minimum Cash Reserve check (spot solvency without leverage)
        let required_cash_usd = (total_portfolio_usd as f64 * (policy.min_cash_bps as f64 / 10_000.0)).round() as u64;
        let total_buy_outflow: u64 = trades.iter().filter(|t| t.is_buy).map(|t| t.trade_value).sum();
        if available_cash_usd < total_buy_outflow || (available_cash_usd - total_buy_outflow) < required_cash_usd {
            return RiskAssessment::Rejected {
                reason: format!(
                    "Cash reserve breach: available cash ${} is insufficient for buy outlay ${} while maintaining minimum required reserve ${} ({} bps)",
                    available_cash_usd, total_buy_outflow, required_cash_usd, policy.min_cash_bps
                ),
            };
        }

        // 4. Stop conditions check
        if let Err(reason) =
            validate_stop_conditions(trades, positions, policy.stop_loss_bps as u16)
        {
            return RiskAssessment::Rejected { reason };
        }

        // All checks passed
        RiskAssessment::Approved
    }
}
