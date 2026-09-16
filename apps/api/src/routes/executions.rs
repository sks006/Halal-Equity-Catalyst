//! Executions route handlers.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{
    error::ApiError, models::ExecutionModel, repositories::ExecutionRepository, state::AppState,
};

pub async fn list_executions_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ExecutionModel>>, ApiError> {
    let repo = ExecutionRepository::new(state.db_pool.clone());
    let executions = repo.list_all().await?;
    Ok(Json(executions))
}

pub async fn list_vault_executions_handler(
    State(state): State<Arc<AppState>>,
    Path(vault_address): Path<String>,
) -> Result<Json<Vec<ExecutionModel>>, ApiError> {
    let repo = ExecutionRepository::new(state.db_pool.clone());
    let executions = repo.list_by_vault(&vault_address).await?;
    Ok(Json(executions))
}
