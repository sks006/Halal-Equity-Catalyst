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

pub fn is_bullish_event(event_type: &str) -> bool {
    matches!(event_type, "earnings_beat" | "product_launch" | "guidance_raised")
}

pub fn is_bearish_event(event_type: &str) -> bool {
    matches!(event_type, "earnings_miss" | "regulatory_action" | "guidance_lowered")
}

pub fn is_emergency_event(event_type: &str) -> bool {
    matches!(event_type, "circuit_breaker" | "exploit_detected" | "extreme_volatility")
}

/// Shifts target allocation weights based on policy evaluation
pub fn evaluate_policy_event(
    target: &AllocationTarget,
    event_type: &str,
    symbol: &str,
    sentiment_score: f32,
    policy: &PolicyDefinition,
) -> Result<AllocationTarget, crate::validation::ValidationError> {
    use crate::types::AssetWeight;
    use crate::constants::MAX_BPS;

    let signal = evaluate_event_signal(event_type, sentiment_score, policy)
        .unwrap_or(SignalType::Neutral);

    match signal {
        SignalType::EmergencyExit => {
            AllocationTarget::new(vec![AssetWeight {
                symbol: "USDC".to_string(),
                target_weight: BasisPoints(MAX_BPS),
            }])
        }
        SignalType::Bullish => {
            let mut weights = target.weights.clone();
            let delta = 500; // 5.00%
            let mut sym_idx = None;
            let mut usdc_idx = None;

            for (i, w) in weights.iter().enumerate() {
                if w.symbol == symbol {
                    sym_idx = Some(i);
                }
                if w.symbol == "USDC" {
                    usdc_idx = Some(i);
                }
            }

            if let (Some(s_i), Some(u_i)) = (sym_idx, usdc_idx) {
                let new_sym = (weights[s_i].target_weight.0 + delta).min(MAX_BPS);
                let new_usdc = weights[u_i].target_weight.0.saturating_sub(delta);
                weights[s_i].target_weight = BasisPoints(new_sym);
                weights[u_i].target_weight = BasisPoints(new_usdc);
            }

            AllocationTarget::new(weights)
        }
        SignalType::Bearish => {
            let mut weights = target.weights.clone();
            let delta = 500;
            let mut sym_idx = None;
            let mut usdc_idx = None;

            for (i, w) in weights.iter().enumerate() {
                if w.symbol == symbol {
                    sym_idx = Some(i);
                }
                if w.symbol == "USDC" {
                    usdc_idx = Some(i);
                }
            }

            if let (Some(s_i), Some(u_i)) = (sym_idx, usdc_idx) {
                let new_sym = weights[s_i].target_weight.0.saturating_sub(delta);
                let new_usdc = (weights[u_i].target_weight.0 + delta).min(MAX_BPS);
                weights[s_i].target_weight = BasisPoints(new_sym);
                weights[u_i].target_weight = BasisPoints(new_usdc);
            }

            AllocationTarget::new(weights)
        }
        _ => Ok(target.clone()),
    }
}
