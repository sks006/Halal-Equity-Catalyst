//! Router configuration and middleware pipeline.

use axum::{
    http::Uri,
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    error::ApiError,
    routes::{health_handler, ready_handler},
    state::AppState,
};

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .fallback(fallback_handler)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn fallback_handler(uri: Uri) -> ApiError {
    ApiError::NotFound(format!("Route not found: {}", uri.path()))
}
