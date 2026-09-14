//! Shared application state with database pool and redis client.

use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::Config;

#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub start_time: DateTime<Utc>,
    pub is_ready: AtomicBool,
    pub db_pool: Pool,
    pub redis_client: Option<redis::Client>,
}

impl AppState {
    pub fn new(config: Config, db_pool: Pool, redis_client: Option<redis::Client>) -> Self {
        Self {
            config,
            start_time: Utc::now(),
            is_ready: AtomicBool::new(true),
            db_pool,
            redis_client,
        }
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
}
