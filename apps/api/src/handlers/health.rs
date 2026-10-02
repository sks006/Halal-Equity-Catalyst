//! Health check and system readiness request handlers.
//!
//! Conforms to Single Responsibility Principle (SRP):
//! Focuses exclusively on HTTP request/response transformation for health endpoints,
//! delegating health monitoring and readiness checks to dedicated domain services.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_seconds: u64,
    pub cluster: String,
}

#[derive(Debug, Serialize)]
pub struct ReadyResponse {
    pub ready: bool,
    pub database: String,
    pub redis: String,
    pub uptime_seconds: u64,
}

/// Handler for GET /health - Liveness probe
pub async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let response = HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.uptime_seconds(),
        cluster: state.config.solana_cluster.clone(),
    };

    (StatusCode::OK, Json(response))
}

/// Handler for GET /ready - Readiness probe delegating to `ReadinessChecker` abstraction (DIP & SRP)
pub async fn ready_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let probe = state
        .readiness_checker()
        .check_readiness(state.is_ready())
        .await;

    let status_code = if probe.ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = ReadyResponse {
        ready: probe.ready,
        database: probe.database,
        redis: probe.redis,
        uptime_seconds: state.uptime_seconds(),
    };

    (status_code, Json(response))
}

/// Handler for GET /health/detailed - Full component health monitoring report
pub async fn detailed_health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let report = state.health_monitor().run_full_check().await;
    let status_code = match report.overall_status {
        crate::services::HealthStatus::Healthy => StatusCode::OK,
        crate::services::HealthStatus::Degraded => StatusCode::OK,
        crate::services::HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(report))
}
