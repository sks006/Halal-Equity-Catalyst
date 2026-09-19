//! Equity Catalyst REST API library.

pub mod config;
pub mod engines;
pub mod error;
pub mod middleware;
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
    let solana_service = Some(services::SolanaService::new_with_fallbacks(
        &config.solana_rpc_url,
        config.solana_fallback_rpc_urls.clone(),
        &config.solana_ws_url,
        None,
        None,
        std::time::Duration::from_millis(config.solana_rpc_timeout_ms),
    ));

    let pyth_client = Arc::new(equity_catalyst_pyth::PythClient::new(
        &config.pyth_hermes_url,
    ));
    let portfolio_repo = repositories::PortfolioRepository::new(pool.clone());
    let oracle_service = Arc::new(services::OracleService::new(
        pyth_client,
        Some(portfolio_repo.clone()),
    ));

    let jupiter_client = Arc::new(equity_catalyst_jupiter::JupiterClient::new(
        &config.jupiter_api_url,
    ));
    let risk_engine = Arc::new(engines::risk_engine::RiskEngine::new());
    let vault_repo = repositories::VaultRepository::new(pool.clone());
    let policy_repo = repositories::PolicyRepository::new(pool.clone());
    let execution_repo = repositories::ExecutionRepository::new(pool.clone());
    let quote_service = Arc::new(services::QuoteExecutionService::new(
        jupiter_client,
        risk_engine,
        Some(vault_repo),
        Some(policy_repo),
        Some(portfolio_repo),
        Some(execution_repo),
    ));

    let state = Arc::new(
        AppState::new(config, pool, redis_client, solana_service)
            .with_oracle_service(oracle_service)
            .with_quote_service(quote_service),
    );
    create_router(state)
}
