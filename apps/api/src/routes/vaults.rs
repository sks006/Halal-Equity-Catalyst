//! Vaults route handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{error::ApiError, models::VaultModel, repositories::VaultRepository, state::AppState};

pub async fn list_vaults_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<VaultModel>>, ApiError> {
    let repo = VaultRepository::new(state.db_pool.clone());
    let vaults = repo.list_all().await?;
    Ok(Json(vaults))
}

pub async fn get_vault_handler(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> Result<Json<VaultModel>, ApiError> {
    let repo = VaultRepository::new(state.db_pool.clone());
    let vault = repo
        .find_by_address(&address)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Vault not found: {}", address)))?;
    Ok(Json(vault))
}

pub async fn create_vault_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VaultModel>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = VaultRepository::new(state.db_pool.clone());
    let created = repo.create(&payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}
