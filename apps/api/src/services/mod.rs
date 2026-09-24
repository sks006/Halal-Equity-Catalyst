//! Backend domain services for Equity Catalyst.

pub mod asset_subscription_watcher;
pub mod execution_engine_service;
pub mod health_monitor;
pub mod market_data_store;
pub mod oracle_service;
pub mod quote_service;
pub mod solana_service;
pub mod vault_service;

pub use asset_subscription_watcher::{AssetSubscriptionWatcher, SubscriptionWatcherConfig};
pub use equity_catalyst_pyth::{DynamicStreamManager, PriceUpdateSink, StreamManagerConfig};
pub use execution_engine_service::{ExecutionEngineService, ExecutionOutcome, ExecutionRequest};
pub use health_monitor::{ComponentHealth, HealthMonitor, HealthStatus, SystemHealthReport};
pub use market_data_store::{
    calculate_scaled_int, MarketDataError, MarketDataStore, MarketPriceUpdate, PriceFreshness,
};
pub use oracle_service::OracleService;
pub use quote_service::{QuoteExecutionRequest, QuoteExecutionService, QuoteExecutionVerdict};
pub use solana_service::SolanaService;
pub use vault_service::VaultService;
