//! Production metrics and telemetry endpoint for Equity Catalyst API.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::state::AppState;

/// System and pipeline metrics report.
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub service: &'static str,
    pub version: &'static str,
    pub environment: String,
    pub cluster: String,
    pub uptime_seconds: u64,
    pub memory_rss_bytes: Option<u64>,
    pub max_trade_size_usd: u64,
    pub max_slippage_bps: u16,
    pub rate_limit_rpm: u32,
    pub market_data_subscriptions: usize,
    pub read_only_mode: bool,
}

/// Handler for GET /metrics
pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rss = read_process_rss_bytes();
    let subscriptions_count = if let Some(ref store) = state.market_data_store {
        store.len().await
    } else {
        0
    };

    let metrics = MetricsResponse {
        service: "equity-catalyst-api",
        version: env!("CARGO_PKG_VERSION"),
        environment: state.config.environment.to_string(),
        cluster: state.config.solana_cluster.clone(),
        uptime_seconds: state.uptime_seconds(),
        memory_rss_bytes: rss,
        max_trade_size_usd: state.config.max_trade_size_usd,
        max_slippage_bps: state.config.max_slippage_bps,
        rate_limit_rpm: state.config.rate_limit_requests_per_minute,
        market_data_subscriptions: subscriptions_count,
        read_only_mode: state.config.read_only,
    };

    (StatusCode::OK, Json(metrics))
}

fn read_process_rss_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = statm.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(resident_pages) = parts[1].parse::<u64>() {
                    let page_size = 4096u64; // Standard Linux page size
                    return Some(resident_pages * page_size);
                }
            }
        }
    }
    None
}
