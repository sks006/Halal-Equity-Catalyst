//! Risk Engine acting as the second line of defense before transaction submission.

pub mod exposure;
pub mod limits;
pub mod ltv;
pub mod stops;

pub use exposure::validate_position_exposure;
pub use limits::validate_trade_limits;
pub use ltv::validate_ltv;
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

    /// Evaluates proposed trades through the complete risk pipeline:
    /// Proposed Action -> Position Exposure -> Policy Limits -> LTV -> Stop Conditions -> Approve / Reject
    pub fn evaluate_proposed_trades(
        &self,
        trades: &[RebalanceTrade],
        positions: &[PortfolioModel],
        policy: &PolicyModel,
        total_portfolio_usd: u64,
        total_debt_usd: u64,
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

        // 3. LTV check
        if let Err(reason) = validate_ltv(
            total_debt_usd,
            total_portfolio_usd,
            policy.max_ltv_bps as u16,
        ) {
            return RiskAssessment::Rejected { reason };
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
