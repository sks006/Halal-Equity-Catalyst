//! Equity Catalyst REST API library.

pub mod config;
pub mod engines;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod pipeline;
pub mod repositories;
pub mod router;
pub mod routes;
pub mod services;
pub mod state;
pub mod workers;

pub use config::{sanitize_connection_url, Config, ConfigError};
pub use engines::{
    load_production_signer, AllocationProposal, AnchorExecutionConnector, AnchorExecutionError,
    CanonicalExecutionPayload, DecisionEngine, DeterministicPolicyAuthorizer, DevTestSigner,
    ExecutionAuthorization, ExecutionPlan, ExecutionPlanner, ExecutionSigner, ExternalSigner,
    HsmSigner, IdempotencyRecord, IdempotencyStatus, IdempotencyTracker, KeypairSigner, KmsSigner,
    OracleReferenceInfo, PlannerError, PolicyRejectionReason, PolicyValidationOutcome,
    PreconditionParameters, RemoteHsmSigner, SignedTransaction, SignerError, SigningLifecycle,
    TransactionBuilder, TransactionSignerService, TransactionSubmitter, UnavailableSigner,
    UnsignedTransaction, ValidationContext,
};
pub use equity_catalyst_pyth::{
    DynamicStreamManager, PriceUpdateSink, PythSubscription, StreamManagerConfig, SubscriptionSet,
};
pub use error::ApiError;
pub use pipeline::{apply_middleware_pipeline, MiddlewarePipeline};
pub use repositories::{
    DbcPoolReader, DbcPoolRepositoryTrait, DbcPoolWriter, EventReader, EventRepositoryTrait,
    EventWriter, ExecutionReader, ExecutionRecorder, ExecutionRepositoryTrait,
    InMemoryDbcPoolRepository, InMemoryEventRepository, InMemoryExecutionRepository,
    InMemoryPolicyRepository, InMemoryPortfolioRepository, InMemoryVaultRepository, PolicyReader,
    PolicyRepositoryTrait, PolicyWriter, PortfolioReader, PortfolioRepositoryTrait,
    PortfolioWriter, VaultReader, VaultRepositoryTrait, VaultWriter,
};
pub use router::create_router;
pub use routes::{get_market_data_handler, market_data_ws_handler, MarketDataResponse};
pub use services::{
    AssetSubscriptionWatcher, ComponentHealth, DefaultReadinessChecker, HealthCheckable,
    HealthMonitor, HealthStatus, InMemoryReadinessChecker, MarketDataError, MarketDataStore,
    MarketPriceUpdate, OracleService, PriceFeedProvider, PriceFreshness, QuoteExecutionRequest,
    QuoteExecutionService, QuoteExecutionVerdict, QuoteProvider, ReadinessChecker,
    ReadinessProbeResult, SolanaService, SubscriptionWatcherConfig, SystemHealthReport,
    VaultService,
};
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

    let pyth_client = Arc::new(equity_catalyst_pyth::PythClient::with_api_key(
        &config.pyth.hermes_url,
        config.pyth.api_key.clone(),
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

    let market_data_store = Arc::new(services::MarketDataStore::new());
    let state = Arc::new(
        AppState::new(config, pool, redis_client, solana_service)
            .with_oracle_service(oracle_service)
            .with_quote_service(quote_service)
            .with_market_data_store(market_data_store),
    );
    create_router(state)
}

/// Builds the API router configured with in-memory repositories (Liskov Substitution Principle).
/// Enables zero-dependency unit and integration testing without PostgreSQL or Redis running.
pub fn build_app_in_memory(config: Config) -> Router {
    let pool = create_db_pool(&config.database_url).unwrap_or_else(|_| {
        let mut db_cfg = DbConfig::new();
        db_cfg.url = Some("postgres://dummy:dummy@localhost:5432/dummy".to_string());
        db_cfg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap()
    });
    let state = Arc::new(AppState::new_in_memory(config, pool, None, None));
    create_router(state)
}
