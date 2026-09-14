//! Condition evaluations for policy engine triggers.

use equity_catalyst_shared::{calculate_drift, types::BasisPoints};

/// Checks if an external sentiment score exceeds the specified trigger threshold.
pub fn is_sentiment_trigger(score: Option<f64>, threshold: f64) -> bool {
    match score {
        Some(s) => s.abs() >= threshold,
        None => false,
    }
}

/// Checks if sentiment indicates a strong positive market/earnings surprise.
pub fn is_bullish_sentiment(score: Option<f64>, threshold: f64) -> bool {
    match score {
        Some(s) => s >= threshold,
        None => false,
    }
}

/// Checks if sentiment indicates a strong negative market/earnings surprise.
pub fn is_bearish_sentiment(score: Option<f64>, threshold: f64) -> bool {
    match score {
        Some(s) => s <= -threshold,
        None => false,
    }
}

/// Checks if portfolio asset drift exceeds allowable rebalance threshold.
pub fn is_drift_trigger(current_bps: u16, target_bps: u16, threshold_bps: u16) -> bool {
    calculate_drift(BasisPoints(current_bps), BasisPoints(target_bps)) >= BasisPoints(threshold_bps)
}

/// Checks if an event indicates a protocol emergency halt.
pub fn is_emergency_event(event_type: &str) -> bool {
    let upper = event_type.to_uppercase();
    upper.contains("EMERGENCY")
        || upper.contains("HALT")
        || upper.contains("EXPLOIT")
        || upper.contains("CIRCUIT_BREAKER")
}
