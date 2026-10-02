//! Risk Policy request handlers.
//!
//! Conforms to Single Responsibility Principle (SRP) and Dependency Inversion Principle (DIP):
//! Exclusively handles HTTP layer mapping, delegating policy persistence to `PolicyRepositoryTrait`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{error::ApiError, models::PolicyModel, state::AppState};

pub async fn list_policies_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PolicyModel>>, ApiError> {
    let policies = state.policy_repo().list_all().await?;
    Ok(Json(policies))
}

pub async fn get_policy_handler(
    State(state): State<Arc<AppState>>,
    Path(vault_address): Path<String>,
) -> Result<Json<PolicyModel>, ApiError> {
    let policy = state
        .policy_repo()
        .find_by_vault(&vault_address)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Policy not found for vault: {}", vault_address))
        })?;
    Ok(Json(policy))
}

pub async fn create_or_update_policy_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PolicyModel>,
) -> Result<impl IntoResponse, ApiError> {
    let saved = state.policy_repo().upsert(&payload).await?;
    Ok((StatusCode::CREATED, Json(saved)))
}
