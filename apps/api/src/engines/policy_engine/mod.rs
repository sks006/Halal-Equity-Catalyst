//! Policy Engine evaluating events, conditions, rules, signals, and target allocations.

pub mod allocation;
pub mod conditions;
pub mod rules;
pub mod signals;

pub use allocation::calculate_target_allocation;
pub use conditions::*;
pub use rules::{match_rule, PolicyRule};
pub use signals::{generate_signal, PolicySignal};

use equity_catalyst_shared::{
    allocation::RebalanceTrade,
    calculate_drift,
    types::{AllocationTarget, BasisPoints},
};

use crate::{
    error::ApiError,
    models::{EventModel, PolicyModel, PortfolioModel},
};

#[derive(Debug, Clone)]
pub struct PolicyEvaluationResult {
    pub rule: PolicyRule,
    pub signal: PolicySignal,
    pub target_allocation: AllocationTarget,
    pub proposed_trades: Vec<RebalanceTrade>,
}

#[derive(Debug, Clone, Default)]
pub struct PolicyEngine;

impl PolicyEngine {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates an event against the policy to determine target allocations and proposed trades.
    pub fn evaluate(
        &self,
        event: &EventModel,
        policy: &PolicyModel,
        positions: &[PortfolioModel],
        total_value_usd: u64,
    ) -> Result<PolicyEvaluationResult, ApiError> {
        // 1. Calculate maximum drift across current positions
        let max_drift_bps = positions
            .iter()
            .map(|p| {
                calculate_drift(
                    BasisPoints(p.current_weight_bps as u16),
                    BasisPoints(p.target_weight_bps as u16),
                )
                .0
            })
            .max()
            .unwrap_or(0);

        // 2. Match policy rule
        let rule = match_rule(event, policy, max_drift_bps);

        // 3. Generate domain signal
        let signal = generate_signal(rule.clone());

        // 4. Calculate target allocation and proposed trades
        let (target_allocation, proposed_trades) = calculate_target_allocation(
            &signal,
            positions,
            total_value_usd,
            policy.rebalance_threshold_bps as u16,
        )?;

        Ok(PolicyEvaluationResult {
            rule,
            signal,
            target_allocation,
            proposed_trades,
        })
    }
}
