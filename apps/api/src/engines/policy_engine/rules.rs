//! Policy rules matching incoming events to operational rule categories.

use crate::models::{EventModel, PolicyModel};
use super::conditions::{
    is_bearish_sentiment, is_bullish_sentiment, is_emergency_event,
};

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyRule {
    EmergencyHalt,
    EarningsBeat { symbol: String, sentiment: f64 },
    EarningsMiss { symbol: String, sentiment: f64 },
    DriftRebalance { max_drift_bps: u16 },
    NoOp,
}

/// Evaluates which policy rule applies given an incoming event and active policy.
pub fn match_rule(
    event: &EventModel,
    policy: &PolicyModel,
    max_drift_bps: u16,
) -> PolicyRule {
    if !policy.is_active {
        return PolicyRule::NoOp;
    }

    if is_emergency_event(&event.event_type) {
        return PolicyRule::EmergencyHalt;
    }

    let symbol = event
        .payload
        .get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("NVDA")
        .to_string();

    const SENTIMENT_THRESHOLD: f64 = 0.50;

    if event.event_type.contains("EARNINGS") || event.event_type.contains("NEWS") {
        if is_bullish_sentiment(event.sentiment_score, SENTIMENT_THRESHOLD) {
            return PolicyRule::EarningsBeat {
                symbol,
                sentiment: event.sentiment_score.unwrap_or(0.0),
            };
        } else if is_bearish_sentiment(event.sentiment_score, SENTIMENT_THRESHOLD) {
            return PolicyRule::EarningsMiss {
                symbol,
                sentiment: event.sentiment_score.unwrap_or(0.0),
            };
        }
    }

    if max_drift_bps >= policy.rebalance_threshold_bps as u16 {
        return PolicyRule::DriftRebalance { max_drift_bps };
    }

    PolicyRule::NoOp
}
