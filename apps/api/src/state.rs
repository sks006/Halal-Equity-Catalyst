//! Shared application state with database pool and service container.
//!
//! Follows Dependency Inversion Principle (DIP):
//! Handlers depend on repository and service traits injected into `AppState`,
//! rather than hardcoded PostgreSQL pools and concrete repository instances.

use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::{
    config::Config,
    repositories::{
        DbcPoolRepository, DbcPoolRepositoryTrait, EventRepository, EventRepositoryTrait,
        ExecutionRepository, ExecutionRepositoryTrait, InMemoryDbcPoolRepository,
        InMemoryEventRepository, InMemoryExecutionRepository, InMemoryPolicyRepository,
        InMemoryVaultRepository, PolicyRepository, PolicyRepositoryTrait, VaultRepository,
        VaultRepositoryTrait,
    },
    services::{
        DefaultReadinessChecker, HealthMonitor, InMemoryReadinessChecker, MarketDataStore,
        OracleService, QuoteExecutionService, ReadinessChecker, SolanaService, VaultService,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub start_time: DateTime<Utc>,
    pub is_ready: Arc<AtomicBool>,
    pub db_pool: Pool,
    pub redis_client: Option<redis::Client>,
    pub solana_service: Option<SolanaService>,
    pub oracle_service: Option<Arc<OracleService>>,
    pub quote_service: Option<Arc<QuoteExecutionService>>,
    pub market_data_store: Option<Arc<MarketDataStore>>,
    pub rate_limiter: Arc<crate::middleware::RateLimiter>,

    // DIP Service & Repository Container
    pub vault_repo: Arc<dyn VaultRepositoryTrait>,
    pub policy_repo: Arc<dyn PolicyRepositoryTrait>,
    pub event_repo: Arc<dyn EventRepositoryTrait>,
    pub execution_repo: Arc<dyn ExecutionRepositoryTrait>,
    pub dbc_pool_repo: Arc<dyn DbcPoolRepositoryTrait>,
    pub readiness_checker: Arc<dyn ReadinessChecker>,
    pub vault_service: Arc<VaultService>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .field("start_time", &self.start_time)
            .field("is_ready", &self.is_ready)
            .field("db_pool", &"Pool")
            .field("redis_client", &self.redis_client)
            .finish()
    }
}

impl AppState {
    pub fn new(
        config: Config,
        db_pool: Pool,
        redis_client: Option<redis::Client>,
        solana_service: Option<SolanaService>,
    ) -> Self {
        let rate_limiter = Arc::new(crate::middleware::RateLimiter::new(
            config.rate_limit_requests_per_minute,
            std::time::Duration::from_secs(60),
        ));

        let vault_repo =
            Arc::new(VaultRepository::new(db_pool.clone())) as Arc<dyn VaultRepositoryTrait>;
        let policy_repo =
            Arc::new(PolicyRepository::new(db_pool.clone())) as Arc<dyn PolicyRepositoryTrait>;
        let event_repo =
            Arc::new(EventRepository::new(db_pool.clone())) as Arc<dyn EventRepositoryTrait>;
        let execution_repo = Arc::new(ExecutionRepository::new(db_pool.clone()))
            as Arc<dyn ExecutionRepositoryTrait>;
        let dbc_pool_repo =
            Arc::new(DbcPoolRepository::new(db_pool.clone())) as Arc<dyn DbcPoolRepositoryTrait>;
        let readiness_checker = Arc::new(DefaultReadinessChecker::new(
            db_pool.clone(),
            redis_client.clone(),
        )) as Arc<dyn ReadinessChecker>;
        let vault_service = Arc::new(VaultService::new_with_repo(vault_repo.clone()));

        Self {
            config,
            start_time: Utc::now(),
            is_ready: Arc::new(AtomicBool::new(true)),
            db_pool,
            redis_client,
            solana_service,
            oracle_service: None,
            quote_service: None,
            market_data_store: None,
            rate_limiter,
            vault_repo,
            policy_repo,
            event_repo,
            execution_repo,
            dbc_pool_repo,
            readiness_checker,
            vault_service,
        }
    }

    /// Creates an AppState configured with in-memory repositories (Liskov Substitution Principle).
    /// Ideal for lightweight integration tests and environments without active PostgreSQL/Redis.
    pub fn new_in_memory(
        config: Config,
        db_pool: Pool,
        redis_client: Option<redis::Client>,
        solana_service: Option<SolanaService>,
    ) -> Self {
        let rate_limiter = Arc::new(crate::middleware::RateLimiter::new(
            config.rate_limit_requests_per_minute,
            std::time::Duration::from_secs(60),
        ));

        let vault_repo = Arc::new(InMemoryVaultRepository::new()) as Arc<dyn VaultRepositoryTrait>;
        let policy_repo =
            Arc::new(InMemoryPolicyRepository::new()) as Arc<dyn PolicyRepositoryTrait>;
        let event_repo = Arc::new(InMemoryEventRepository::new()) as Arc<dyn EventRepositoryTrait>;
        let execution_repo =
            Arc::new(InMemoryExecutionRepository::new()) as Arc<dyn ExecutionRepositoryTrait>;
        let dbc_pool_repo =
            Arc::new(InMemoryDbcPoolRepository::new()) as Arc<dyn DbcPoolRepositoryTrait>;
        let readiness_checker =
            Arc::new(InMemoryReadinessChecker::new()) as Arc<dyn ReadinessChecker>;
        let vault_service = Arc::new(VaultService::new_with_repo(vault_repo.clone()));

        Self {
            config,
            start_time: Utc::now(),
            is_ready: Arc::new(AtomicBool::new(true)),
            db_pool,
            redis_client,
            solana_service,
            oracle_service: None,
            quote_service: None,
            market_data_store: None,
            rate_limiter,
            vault_repo,
            policy_repo,
            event_repo,
            execution_repo,
            dbc_pool_repo,
            readiness_checker,
            vault_service,
        }
    }

    pub fn with_rate_limiter(mut self, rate_limiter: Arc<crate::middleware::RateLimiter>) -> Self {
        self.rate_limiter = rate_limiter;
        self
    }

    pub fn rate_limiter(&self) -> &crate::middleware::RateLimiter {
        &self.rate_limiter
    }

    pub fn with_oracle_service(mut self, oracle_service: Arc<OracleService>) -> Self {
        self.oracle_service = Some(oracle_service);
        self
    }

    pub fn with_quote_service(mut self, quote_service: Arc<QuoteExecutionService>) -> Self {
        self.quote_service = Some(quote_service);
        self
    }

    pub fn with_market_data_store(mut self, market_data_store: Arc<MarketDataStore>) -> Self {
        self.market_data_store = Some(market_data_store);
        self
    }

    pub fn with_vault_repo(mut self, vault_repo: Arc<dyn VaultRepositoryTrait>) -> Self {
        self.vault_service = Arc::new(VaultService::new_with_repo(vault_repo.clone()));
        self.vault_repo = vault_repo;
        self
    }

    pub fn with_policy_repo(mut self, policy_repo: Arc<dyn PolicyRepositoryTrait>) -> Self {
        self.policy_repo = policy_repo;
        self
    }

    pub fn with_event_repo(mut self, event_repo: Arc<dyn EventRepositoryTrait>) -> Self {
        self.event_repo = event_repo;
        self
    }

    pub fn with_execution_repo(
        mut self,
        execution_repo: Arc<dyn ExecutionRepositoryTrait>,
    ) -> Self {
        self.execution_repo = execution_repo;
        self
    }

    pub fn with_dbc_pool_repo(mut self, dbc_pool_repo: Arc<dyn DbcPoolRepositoryTrait>) -> Self {
        self.dbc_pool_repo = dbc_pool_repo;
        self
    }

    pub fn with_readiness_checker(mut self, readiness_checker: Arc<dyn ReadinessChecker>) -> Self {
        self.readiness_checker = readiness_checker;
        self
    }

    pub fn with_vault_service(mut self, vault_service: Arc<VaultService>) -> Self {
        self.vault_service = vault_service;
        self
    }

    pub fn vault_repo(&self) -> &Arc<dyn VaultRepositoryTrait> {
        &self.vault_repo
    }

    pub fn policy_repo(&self) -> &Arc<dyn PolicyRepositoryTrait> {
        &self.policy_repo
    }

    pub fn event_repo(&self) -> &Arc<dyn EventRepositoryTrait> {
        &self.event_repo
    }

    pub fn execution_repo(&self) -> &Arc<dyn ExecutionRepositoryTrait> {
        &self.execution_repo
    }

    pub fn dbc_pool_repo(&self) -> &Arc<dyn DbcPoolRepositoryTrait> {
        &self.dbc_pool_repo
    }

    pub fn readiness_checker(&self) -> &Arc<dyn ReadinessChecker> {
        &self.readiness_checker
    }

    pub fn vault_service(&self) -> &Arc<VaultService> {
        &self.vault_service
    }

    pub fn uptime_seconds(&self) -> u64 {
        let now = Utc::now();
        (now - self.start_time).num_seconds().max(0) as u64
    }

    pub fn set_ready(&self, ready: bool) {
        self.is_ready.store(ready, Ordering::SeqCst);
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready.load(Ordering::SeqCst)
    }

    pub fn health_monitor(&self) -> HealthMonitor {
        HealthMonitor::new(
            self.db_pool.clone(),
            self.redis_client.clone(),
            self.solana_service.clone(),
        )
    }
}
