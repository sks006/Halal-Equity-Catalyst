//! Deterministic portfolio allocation and rebalancing algorithms.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::math::{calculate_basis_points, calculate_drift, compute_bps_amount};
use crate::types::{AllocationTarget, BasisPoints, PositionSnapshot};
use crate::validation::ValidationError;

/// Represents a proposed trade order to restore target allocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceTrade {
    pub symbol: String,
    pub is_buy: bool,
    pub current_value: u64,
    pub target_value: u64,
    pub trade_value: u64,
    pub drift_bps: BasisPoints,
}

/// Determines whether any asset has drifted beyond the rebalance threshold.
pub fn is_rebalance_needed(
    positions: &[PositionSnapshot],
    target: &AllocationTarget,
    total_portfolio_value: u64,
    threshold_bps: BasisPoints,
) -> bool {
    if total_portfolio_value == 0 {
        return false;
    }

    let current_values: HashMap<&str, u64> = positions
        .iter()
        .map(|p| (p.symbol.as_str(), p.current_value_usd))
        .collect();

    for weight in &target.weights {
        let current_val = *current_values.get(weight.symbol.as_str()).unwrap_or(&0);
        if let Ok(current_bps) = calculate_basis_points(current_val, total_portfolio_value) {
            let drift = calculate_drift(current_bps, weight.target_weight);
            if drift >= threshold_bps {
                return true;
            }
        }
    }

    false
}

/// Compute complete rebalancing trade plan to realign portfolio with target weights.
pub fn calculate_rebalance_plan(
    positions: &[PositionSnapshot],
    target: &AllocationTarget,
    total_portfolio_value: u64,
    threshold_bps: BasisPoints,
) -> Result<Vec<RebalanceTrade>, ValidationError> {
    if total_portfolio_value == 0 {
        return Err(ValidationError::ZeroPortfolioValue);
    }

    let mut current_values: HashMap<&str, u64> = positions
        .iter()
        .map(|p| (p.symbol.as_str(), p.current_value_usd))
        .collect();

    let mut trades = Vec::new();

    for weight in &target.weights {
        let current_val = current_values.remove(weight.symbol.as_str()).unwrap_or(0);
        let target_val = compute_bps_amount(total_portfolio_value, weight.target_weight)?;

        let current_bps = calculate_basis_points(current_val, total_portfolio_value)?;
        let drift = calculate_drift(current_bps, weight.target_weight);

        if drift >= threshold_bps {
            let (is_buy, trade_value) = if target_val >= current_val {
                (true, target_val - current_val)
            } else {
                (false, current_val - target_val)
            };

            trades.push(RebalanceTrade {
                symbol: weight.symbol.clone(),
                is_buy,
                current_value: current_val,
                target_value: target_val,
                trade_value,
                drift_bps: drift,
            });
        }
    }

    // Any asset currently held that is NOT in target allocation should be sold off (target = 0)
    for (symbol, current_val) in current_values {
        if current_val > 0 {
            let current_bps = calculate_basis_points(current_val, total_portfolio_value)?;
            trades.push(RebalanceTrade {
                symbol: symbol.to_string(),
                is_buy: false,
                current_value: current_val,
                target_value: 0,
                trade_value: current_val,
                drift_bps: current_bps,
            });
        }
    }

    Ok(trades)
}
