//! Deterministic policy engine definitions and condition evaluation.

use serde::{Deserialize, Serialize};

use crate::constants::{DEFAULT_REBALANCE_THRESHOLD_BPS, DEFAULT_STOP_LOSS_BPS, DEFAULT_TAKE_PROFIT_BPS};
use crate::types::{AllocationTarget, BasisPoints, RiskLimits, SignalType};

/// Configured policy definition associated with a vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDefinition {
    pub vault_id: String,
    pub risk_limits: RiskLimits,
    pub target_allocation: Option<AllocationTarget>,
    pub stop_loss_bps: BasisPoints,
    pub take_profit_bps: BasisPoints,
    pub rebalance_threshold_bps: BasisPoints,
    pub is_active: bool,
}

impl Default for PolicyDefinition {
    fn default() -> Self {
        Self {
            vault_id: String::new(),
            risk_limits: RiskLimits::default(),
            target_allocation: None,
            stop_loss_bps: BasisPoints(DEFAULT_STOP_LOSS_BPS),
            take_profit_bps: BasisPoints(DEFAULT_TAKE_PROFIT_BPS),
            rebalance_threshold_bps: BasisPoints(DEFAULT_REBALANCE_THRESHOLD_BPS),
            is_active: true,
        }
    }
}

/// Evaluates a market or earnings event to determine whether it triggers a policy action.
pub fn evaluate_event_signal(
    event_type: &str,
    sentiment_score: f32, // -1.0 to 1.0
    policy: &PolicyDefinition,
) -> Option<SignalType> {
    if !policy.is_active {
        return None;
    }

    match event_type {
        "earnings_beat" if sentiment_score > 0.2 => Some(SignalType::Bullish),
        "earnings_miss" if sentiment_score < -0.2 => Some(SignalType::Bearish),
        "extreme_volatility" | "circuit_breaker" => Some(SignalType::EmergencyExit),
        "periodic_rebalance" => Some(SignalType::RebalanceRequired),
        _ => {
            if sentiment_score > 0.5 {
                Some(SignalType::Bullish)
            } else if sentiment_score < -0.5 {
                Some(SignalType::Bearish)
            } else {
                Some(SignalType::Neutral)
            }
        }
    }
}
