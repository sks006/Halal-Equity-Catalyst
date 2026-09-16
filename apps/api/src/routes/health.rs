//! Health check and system status endpoints.

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

/// Handler for GET /ready - Readiness probe validating DB & Redis connectivity
pub async fn ready_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut db_status = "unhealthy";
    let mut redis_status = "disabled";
    let mut is_healthy = state.is_ready();

    // Check Postgres
    match state.db_pool.get().await {
        Ok(client) => match client.execute("SELECT 1", &[]).await {
            Ok(_) => db_status = "healthy",
            Err(_) => is_healthy = false,
        },
        Err(_) => is_healthy = false,
    }

    // Check Redis
    if let Some(ref client) = state.redis_client {
        match client.get_multiplexed_async_connection().await {
            Ok(mut conn) => match redis::cmd("PING").query_async::<String>(&mut conn).await {
                Ok(resp) if resp == "PONG" => redis_status = "healthy",
                _ => is_healthy = false,
            },
            Err(_) => is_healthy = false,
        }
    }

    let status_code = if is_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = ReadyResponse {
        ready: is_healthy,
        database: db_status.to_string(),
        redis: redis_status.to_string(),
        uptime_seconds: state.uptime_seconds(),
    };

    (status_code, Json(response))
}

/// Handler for GET /health/detailed - Full 8-component health monitoring report
pub async fn detailed_health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let report = state.health_monitor().run_full_check().await;
    let status_code = match report.overall_status {
        crate::services::HealthStatus::Healthy => StatusCode::OK,
        crate::services::HealthStatus::Degraded => StatusCode::OK,
        crate::services::HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(report))
}
