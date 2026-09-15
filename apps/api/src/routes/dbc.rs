//! Meteora DBC configuration endpoints.

use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    engines::dbc_engine::{DbcConfigRequest, DbcEngine},
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
