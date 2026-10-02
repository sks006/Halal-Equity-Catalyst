//! Repository abstractions and traits defining data access contracts.
//!
//! Follows SOLID principles:
//! - **Interface Segregation Principle (ISP)**: Segregated into fine-grained reader and writer traits.
//! - **Liskov Substitution Principle (LSP)**: Any implementation (PostgreSQL, InMemory, Mock, Cached)
//!   can be substituted without breaking caller invariants.
//! - **Dependency Inversion Principle (DIP)**: Domain services and handlers depend on these traits
//!   rather than concrete database structs.

use async_trait::async_trait;

use crate::{
    error::ApiError,
    models::{
        dbc_pool::{CreateDbcPoolRequest, DbcPoolModel},
        EventModel, ExecutionModel, PolicyModel, PortfolioModel, VaultModel,
    },
};

// ============================================================================
// Vault Repository Traits (ISP: Segregated Reader & Writer)
// ============================================================================

/// Read-only queries for Vaults.
#[async_trait]
pub trait VaultReader: Send + Sync {
    /// Retrieves a vault by its Solana base58 address.
    async fn find_by_address(&self, address: &str) -> Result<Option<VaultModel>, ApiError>;

    /// Lists all vaults in the system.
    async fn list_all(&self) -> Result<Vec<VaultModel>, ApiError>;
}

/// Mutation operations for Vaults.
#[async_trait]
pub trait VaultWriter: Send + Sync {
    /// Creates and persists a new vault.
    async fn create(&self, vault: &VaultModel) -> Result<VaultModel, ApiError>;

    /// Updates share and deposit totals.
    async fn update_totals(
        &self,
        address: &str,
        total_shares: u64,
        total_deposits: u64,
    ) -> Result<(), ApiError>;

    /// Toggles the emergency paused flag on a vault.
    async fn set_paused(&self, address: &str, is_paused: bool) -> Result<(), ApiError>;
}

/// Unified Vault repository trait composing reader and writer contracts.
pub trait VaultRepositoryTrait: VaultReader + VaultWriter {}
impl<T: VaultReader + VaultWriter> VaultRepositoryTrait for T {}

// ============================================================================
// Policy Repository Traits (ISP: Segregated Reader & Writer)
// ============================================================================

/// Read-only queries for Risk Policies.
#[async_trait]
pub trait PolicyReader: Send + Sync {
    /// Retrieves the policy configured for a given vault address.
    async fn find_by_vault(&self, vault_address: &str) -> Result<Option<PolicyModel>, ApiError>;

    /// Lists all policies.
    async fn list_all(&self) -> Result<Vec<PolicyModel>, ApiError>;
}

/// Mutation operations for Risk Policies.
#[async_trait]
pub trait PolicyWriter: Send + Sync {
    /// Inserts or updates a risk policy.
    async fn upsert(&self, policy: &PolicyModel) -> Result<PolicyModel, ApiError>;
}

/// Unified Policy repository trait composing reader and writer contracts.
pub trait PolicyRepositoryTrait: PolicyReader + PolicyWriter {}
impl<T: PolicyReader + PolicyWriter> PolicyRepositoryTrait for T {}

// ============================================================================
// Execution Repository Traits (ISP: Segregated Reader & Recorder)
// ============================================================================

/// Read-only queries for Execution audit logs.
#[async_trait]
pub trait ExecutionReader: Send + Sync {
    /// Lists all executions globally.
    async fn list_all(&self) -> Result<Vec<ExecutionModel>, ApiError>;

    /// Lists executions belonging to a specific vault.
    async fn list_by_vault(&self, vault_address: &str) -> Result<Vec<ExecutionModel>, ApiError>;
}

/// Write/Audit recording operations for Executions.
#[async_trait]
pub trait ExecutionRecorder: Send + Sync {
    /// Inserts a new execution record into the audit log.
    async fn create(&self, execution: &ExecutionModel) -> Result<ExecutionModel, ApiError>;
}

/// Unified Execution repository trait composing reader and recorder contracts.
pub trait ExecutionRepositoryTrait: ExecutionReader + ExecutionRecorder {}
impl<T: ExecutionReader + ExecutionRecorder> ExecutionRepositoryTrait for T {}

// ============================================================================
// Event Repository Traits (ISP: Segregated Reader & Writer)
// ============================================================================

/// Read-only queries for On-Chain Events.
#[async_trait]
pub trait EventReader: Send + Sync {
    /// Lists all events globally.
    async fn list_all(&self) -> Result<Vec<EventModel>, ApiError>;

    /// Lists pending (unprocessed) events.
    async fn find_pending(&self) -> Result<Vec<EventModel>, ApiError>;

    /// Lists events for a specific vault.
    async fn list_by_vault(&self, vault_address: &str) -> Result<Vec<EventModel>, ApiError>;
}

/// Write operations for Events.
#[async_trait]
pub trait EventWriter: Send + Sync {
    /// Persists a new captured event.
    async fn create(&self, event: &EventModel) -> Result<EventModel, ApiError>;
}

/// Unified Event repository trait composing reader and writer contracts.
pub trait EventRepositoryTrait: EventReader + EventWriter {}
impl<T: EventReader + EventWriter> EventRepositoryTrait for T {}

// ============================================================================
// Portfolio Repository Traits (ISP: Segregated Reader & Writer)
// ============================================================================

/// Read-only queries for Vault Portfolio positions.
#[async_trait]
pub trait PortfolioReader: Send + Sync {
    /// Lists all asset positions for a given vault.
    async fn list_by_vault(&self, vault_address: &str) -> Result<Vec<PortfolioModel>, ApiError>;
}

/// Write operations for Vault Portfolio positions.
#[async_trait]
pub trait PortfolioWriter: Send + Sync {
    /// Upserts an asset position in the vault portfolio.
    async fn upsert_position(&self, position: &PortfolioModel) -> Result<PortfolioModel, ApiError>;
}

/// Unified Portfolio repository trait composing reader and writer contracts.
pub trait PortfolioRepositoryTrait: PortfolioReader + PortfolioWriter {}
impl<T: PortfolioReader + PortfolioWriter> PortfolioRepositoryTrait for T {}

// ============================================================================
// DBC Pool Repository Traits (ISP: Segregated Reader & Writer)
// ============================================================================

/// Read-only queries for Meteora DBC Pools.
#[async_trait]
pub trait DbcPoolReader: Send + Sync {
    /// Lists all recorded DBC pools.
    async fn list_all(&self) -> Result<Vec<DbcPoolModel>, ApiError>;

    /// Finds a DBC pool by its pool address.
    async fn find_by_pool_address(&self, address: &str) -> Result<Option<DbcPoolModel>, ApiError>;
}

/// Write operations for Meteora DBC Pools.
#[async_trait]
pub trait DbcPoolWriter: Send + Sync {
    /// Creates and persists a newly launched DBC pool.
    async fn create(&self, pool: &CreateDbcPoolRequest) -> Result<DbcPoolModel, ApiError>;
}

/// Unified DBC Pool repository trait composing reader and writer contracts.
pub trait DbcPoolRepositoryTrait: DbcPoolReader + DbcPoolWriter {}
impl<T: DbcPoolReader + DbcPoolWriter> DbcPoolRepositoryTrait for T {}
