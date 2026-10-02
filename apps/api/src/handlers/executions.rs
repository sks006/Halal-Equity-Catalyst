//! Execution audit log request handlers.
//!
//! Conforms to Single Responsibility Principle (SRP) and Dependency Inversion Principle (DIP):
//! Exclusively handles HTTP request processing, delegating audit data access to `ExecutionRepositoryTrait`.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{error::ApiError, models::ExecutionModel, state::AppState};

pub async fn list_executions_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ExecutionModel>>, ApiError> {
    let executions = state.execution_repo().list_all().await?;
    Ok(Json(executions))
}

pub async fn list_vault_executions_handler(
    State(state): State<Arc<AppState>>,
    Path(vault_address): Path<String>,
) -> Result<Json<Vec<ExecutionModel>>, ApiError> {
    let executions = state.execution_repo().list_by_vault(&vault_address).await?;
    Ok(Json(executions))
}
