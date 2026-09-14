//! Shared application state with database pool and redis client.

use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    config::Config,
    services::{HealthMonitor, OracleService, QuoteExecutionService, SolanaService},
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

use std::sync::Arc;

impl AppState {
    pub fn new(
        config: Config,
        db_pool: Pool,
        redis_client: Option<redis::Client>,
        solana_service: Option<SolanaService>,
    ) -> Self {
        Self {
            config,
            start_time: Utc::now(),
            is_ready: Arc::new(AtomicBool::new(true)),
            db_pool,
            redis_client,
            solana_service,
            oracle_service: None,
            quote_service: None,
        }
    }

    pub fn with_oracle_service(mut self, oracle_service: Arc<OracleService>) -> Self {
        self.oracle_service = Some(oracle_service);
        self
    }

    pub fn with_quote_service(mut self, quote_service: Arc<QuoteExecutionService>) -> Self {
        self.quote_service = Some(quote_service);
        self
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
