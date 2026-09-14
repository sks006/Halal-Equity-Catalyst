//! Equity Catalyst REST API library.

pub mod config;
pub mod error;
pub mod router;
pub mod routes;
pub mod state;

pub use config::Config;
pub use error::ApiError;
pub use router::create_router;
pub use state::AppState;

use axum::Router;
use std::sync::Arc;

/// Builds the API router with the provided configuration.
pub fn build_app(config: Config) -> Router {
    let state = Arc::new(AppState::new(config));
    create_router(state)
}
