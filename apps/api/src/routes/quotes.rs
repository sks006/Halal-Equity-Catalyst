//! Quote execution endpoints.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::sync::Arc;

use crate::{
    error::ApiError,
    services::{QuoteExecutionRequest, QuoteExecutionVerdict},
    state::AppState,
};

/// POST /quotes/evaluate - Evaluate a swap quote through the Risk Engine (Quote-only execution mode)
pub async fn evaluate_quote_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<QuoteExecutionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let quote_service = state.quote_service.as_ref().ok_or_else(|| {
        ApiError::InternalServerError("Quote execution service is not configured".to_string())
    })?;

    let verdict: QuoteExecutionVerdict = quote_service.evaluate_quote(&request).await?;
    Ok((StatusCode::OK, Json(verdict)))
}
