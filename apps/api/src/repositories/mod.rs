//! Data access repositories mapping domain entities to PostgreSQL.

pub mod asset_market_data_repository;
pub mod canonical_asset_repository;
pub mod dbc_pool_repository;
pub mod dead_letter_repository;
pub mod event_repository;
pub mod execution_repository;
pub mod policy_repository;
pub mod portfolio_repository;
pub mod vault_repository;

pub use asset_market_data_repository::AssetMarketDataRepository;
pub use canonical_asset_repository::CanonicalAssetRepository;
pub use dbc_pool_repository::DbcPoolRepository;
pub use dead_letter_repository::DeadLetterRepository;
pub use event_repository::EventRepository;
pub use execution_repository::ExecutionRepository;
pub use policy_repository::PolicyRepository;
pub use portfolio_repository::PortfolioRepository;
pub use vault_repository::VaultRepository;


