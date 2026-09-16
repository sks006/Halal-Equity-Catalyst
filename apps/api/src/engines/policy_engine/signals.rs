//! Signal generation mapping policy rules to domain signals.

use super::rules::PolicyRule;
use equity_catalyst_shared::types::SignalType;

#[derive(Debug, Clone, PartialEq)]
pub struct PolicySignal {
    pub signal_type: SignalType,
    pub symbol: Option<String>,
    pub weight_delta_bps: i16,
    pub reason: String,
}

/// Generates a standardized decision signal from a matched policy rule.
pub fn generate_signal(rule: PolicyRule) -> PolicySignal {
    match rule {
        PolicyRule::EmergencyHalt => PolicySignal {
            signal_type: SignalType::EmergencyExit,
            symbol: None,
            weight_delta_bps: 0,
            reason: "Emergency halt condition triggered".to_string(),
        },
        PolicyRule::EarningsBeat { symbol, sentiment } => PolicySignal {
            signal_type: SignalType::Bullish,
            symbol: Some(symbol.clone()),
            weight_delta_bps: 500, // +5.00% target weight expansion
            reason: format!(
                "Bullish event on {} with positive sentiment score {:.2}",
                symbol, sentiment
            ),
        },
        PolicyRule::EarningsMiss { symbol, sentiment } => PolicySignal {
            signal_type: SignalType::Bearish,
            symbol: Some(symbol.clone()),
            weight_delta_bps: -500, // -5.00% target weight reduction
            reason: format!(
                "Bearish event on {} with negative sentiment score {:.2}",
                symbol, sentiment
            ),
        },
        PolicyRule::DriftRebalance { max_drift_bps } => PolicySignal {
            signal_type: SignalType::RebalanceRequired,
            symbol: None,
            weight_delta_bps: 0,
            reason: format!(
                "Asset drift of {} bps exceeds policy rebalance threshold",
                max_drift_bps
            ),
        },
        PolicyRule::NoOp => PolicySignal {
            signal_type: SignalType::Neutral,
            symbol: None,
            weight_delta_bps: 0,
            reason: "No operational policy threshold exceeded".to_string(),
        },
    }
}
