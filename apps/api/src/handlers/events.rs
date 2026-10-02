//! On-chain and market events request handlers.
//!
//! Conforms to Single Responsibility Principle (SRP) and Dependency Inversion Principle (DIP):
//! Exclusively handles HTTP request processing, delegating event persistence to `EventRepositoryTrait`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{error::ApiError, models::EventModel, state::AppState};

pub async fn list_events_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<EventModel>>, ApiError> {
    let events = state.event_repo().list_all().await?;
    Ok(Json(events))
}

pub async fn list_pending_events_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<EventModel>>, ApiError> {
    let events = state.event_repo().find_pending().await?;
    Ok(Json(events))
}

pub async fn list_vault_events_handler(
    State(state): State<Arc<AppState>>,
    Path(vault_address): Path<String>,
) -> Result<Json<Vec<EventModel>>, ApiError> {
    let events = state.event_repo().list_by_vault(&vault_address).await?;
    Ok(Json(events))
}

pub async fn create_event_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<EventModel>,
) -> Result<impl IntoResponse, ApiError> {
    let created = state.event_repo().create(&payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}
