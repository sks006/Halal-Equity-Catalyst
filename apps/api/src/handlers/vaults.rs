//! Vault request handlers.
//!
//! Conforms to Single Responsibility Principle (SRP) and Dependency Inversion Principle (DIP):
//! Exclusively handles HTTP request decoding and response serialization, delegating data access
//! to the injected `VaultRepositoryTrait` abstraction in `AppState`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{error::ApiError, models::VaultModel, state::AppState};

pub async fn list_vaults_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<VaultModel>>, ApiError> {
    let vaults = state.vault_repo().list_all().await?;
    Ok(Json(vaults))
}

pub async fn get_vault_handler(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> Result<Json<VaultModel>, ApiError> {
    let vault = state
        .vault_repo()
        .find_by_address(&address)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Vault not found: {}", address)))?;
    Ok(Json(vault))
}

pub async fn create_vault_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VaultModel>,
) -> Result<impl IntoResponse, ApiError> {
    let created = state.vault_repo().create(&payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}
