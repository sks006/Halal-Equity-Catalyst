//! Oracle price endpoints.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{error::ApiError, state::AppState};

/// GET /oracle/price/:symbol
pub async fn get_price_handler(
    State(state): State<Arc<AppState>>,
    Path(symbol): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let oracle = state.oracle_service.as_ref().ok_or_else(|| {
        ApiError::InternalServerError("Oracle service is not configured".to_string())
    })?;

    let price = oracle.get_normalized_price(&symbol).await?;
    Ok((StatusCode::OK, Json(price)))
}
