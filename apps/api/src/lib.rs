//! Equity Catalyst REST API library.

pub mod config;
pub mod engines;
pub mod error;
pub mod models;
pub mod repositories;
pub mod router;
pub mod routes;
pub mod services;
pub mod state;
pub mod workers;

pub use config::Config;
pub use error::ApiError;
pub use router::create_router;
pub use state::AppState;

use axum::Router;
use deadpool_postgres::{Config as DbConfig, ManagerConfig, Pool, RecyclingMethod, Runtime};
use std::sync::Arc;
use tokio_postgres::NoTls;

/// Creates a PostgreSQL connection pool from a connection string.
pub fn create_db_pool(database_url: &str) -> Result<Pool, Box<dyn std::error::Error>> {
    let mut db_cfg = DbConfig::new();
    db_cfg.url = Some(database_url.to_string());
    db_cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    let pool = db_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
    Ok(pool)
}

/// Builds the API router with the provided state.
pub fn build_app(config: Config, pool: Pool, redis_client: Option<redis::Client>) -> Router {
    let solana_service = Some(services::SolanaService::new(
        &config.solana_rpc_url,
        &config.solana_ws_url,
        None,
        None,
    ));
    let state = Arc::new(AppState::new(config, pool, redis_client, solana_service));
    create_router(state)
}
