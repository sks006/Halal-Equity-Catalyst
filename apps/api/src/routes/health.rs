//! Health check and system status endpoints.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
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

/// Handler for GET /ready - Readiness probe
pub async fn ready_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let ready = state.is_ready();
    let status_code = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = ReadyResponse {
        ready,
        uptime_seconds: state.uptime_seconds(),
    };

    (status_code, Json(response))
}
