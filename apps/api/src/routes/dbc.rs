//! Meteora DBC configuration endpoints.

use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    engines::dbc_engine::{
        ComparisonSimulationRequest, DbcConfigRequest, DbcEngine, DbcSimulationInput, DbcSimulator,
    },
    error::ApiError,
};

/// POST /dbc/configure - Validates and compiles a DBC configuration for tokenized equity launches.
pub async fn configure_dbc_handler(
    Json(request): Json<DbcConfigRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let engine = DbcEngine::new();
    let response = engine
        .configure(request)
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;

    Ok((StatusCode::OK, Json(response)))
}

/// POST /dbc/simulate - Simulates price path, slippage, and graduation for a single DBC configuration.
pub async fn simulate_dbc_handler(
    Json(request): Json<DbcSimulationInput>,
) -> Result<impl IntoResponse, ApiError> {
    let result = DbcSimulator::simulate(&request);
    Ok((StatusCode::OK, Json(result)))
}

/// POST /dbc/simulate/compare - Compares Config A vs Config B vs Default DBC across block orders.
pub async fn compare_dbc_handler(
    Json(request): Json<ComparisonSimulationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let result = DbcSimulator::compare(request);
    Ok((StatusCode::OK, Json(result)))
}

