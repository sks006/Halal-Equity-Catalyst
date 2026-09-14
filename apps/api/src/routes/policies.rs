//! Policies route handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{
    error::ApiError,
    models::PolicyModel,
    repositories::PolicyRepository,
    state::AppState,
};

pub async fn list_policies_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PolicyModel>>, ApiError> {
    let repo = PolicyRepository::new(state.db_pool.clone());
    let policies = repo.list_all().await?;
    Ok(Json(policies))
}

pub async fn get_policy_handler(
    State(state): State<Arc<AppState>>,
    Path(vault_address): Path<String>,
) -> Result<Json<PolicyModel>, ApiError> {
    let repo = PolicyRepository::new(state.db_pool.clone());
    let policy = repo
        .find_by_vault(&vault_address)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Policy not found for vault: {}", vault_address)))?;
    Ok(Json(policy))
}

pub async fn create_or_update_policy_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PolicyModel>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = PolicyRepository::new(state.db_pool.clone());
    let saved = repo.upsert(&payload).await?;
    Ok((StatusCode::CREATED, Json(saved)))
}
