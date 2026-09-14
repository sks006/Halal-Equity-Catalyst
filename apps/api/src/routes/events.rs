//! Events route handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{
    error::ApiError,
    models::EventModel,
    repositories::EventRepository,
    state::AppState,
};

pub async fn list_events_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<EventModel>>, ApiError> {
    let repo = EventRepository::new(state.db_pool.clone());
    let events = repo.list_all().await?;
    Ok(Json(events))
}

pub async fn list_pending_events_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<EventModel>>, ApiError> {
    let repo = EventRepository::new(state.db_pool.clone());
    let events = repo.find_pending().await?;
    Ok(Json(events))
}

pub async fn list_vault_events_handler(
    State(state): State<Arc<AppState>>,
    Path(vault_address): Path<String>,
) -> Result<Json<Vec<EventModel>>, ApiError> {
    let repo = EventRepository::new(state.db_pool.clone());
    let events = repo.list_by_vault(&vault_address).await?;
    Ok(Json(events))
}

pub async fn create_event_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<EventModel>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = EventRepository::new(state.db_pool.clone());
    let created = repo.create(&payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}
