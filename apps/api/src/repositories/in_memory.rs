//! In-memory implementations of repository traits.
//!
//! Follows Liskov Substitution Principle (LSP):
//! These in-memory implementations provide identical behavioral contracts to the PostgreSQL
//! repositories, allowing full system testing, mocking, and offline development without
//! external infrastructure dependencies.

use async_trait::async_trait;
use chrono::Utc;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    error::ApiError,
    models::{
        dbc_pool::{CreateDbcPoolRequest, DbcPoolModel},
        EventModel, ExecutionModel, PolicyModel, PortfolioModel, VaultModel,
    },
    repositories::traits::{
        DbcPoolReader, DbcPoolWriter, EventReader, EventWriter, ExecutionReader, ExecutionRecorder,
        PolicyReader, PolicyWriter, PortfolioReader, PortfolioWriter, VaultReader, VaultWriter,
    },
};

// ============================================================================
// In-Memory Vault Repository
// ============================================================================

#[derive(Clone, Default)]
pub struct InMemoryVaultRepository {
    vaults: Arc<RwLock<HashMap<String, VaultModel>>>,
}

impl InMemoryVaultRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl VaultReader for InMemoryVaultRepository {
    async fn find_by_address(&self, address: &str) -> Result<Option<VaultModel>, ApiError> {
        let read_guard = self.vaults.read().await;
        Ok(read_guard.get(address).cloned())
    }

    async fn list_all(&self) -> Result<Vec<VaultModel>, ApiError> {
        let read_guard = self.vaults.read().await;
        Ok(read_guard.values().cloned().collect())
    }
}

#[async_trait]
impl VaultWriter for InMemoryVaultRepository {
    async fn create(&self, vault: &VaultModel) -> Result<VaultModel, ApiError> {
        let mut write_guard = self.vaults.write().await;
        let mut new_vault = vault.clone();
        new_vault.created_at = Utc::now();
        new_vault.updated_at = Utc::now();
        write_guard.insert(vault.vault_address.clone(), new_vault.clone());
        Ok(new_vault)
    }

    async fn update_totals(
        &self,
        address: &str,
        total_shares: u64,
        total_deposits: u64,
    ) -> Result<(), ApiError> {
        let mut write_guard = self.vaults.write().await;
        if let Some(vault) = write_guard.get_mut(address) {
            vault.total_shares = total_shares;
            vault.total_deposits = total_deposits;
            vault.updated_at = Utc::now();
            Ok(())
        } else {
            Err(ApiError::NotFound(format!("Vault not found: {}", address)))
        }
    }

    async fn set_paused(&self, address: &str, is_paused: bool) -> Result<(), ApiError> {
        let mut write_guard = self.vaults.write().await;
        if let Some(vault) = write_guard.get_mut(address) {
            vault.is_paused = is_paused;
            vault.updated_at = Utc::now();
            Ok(())
        } else {
            Err(ApiError::NotFound(format!("Vault not found: {}", address)))
        }
    }
}

// ============================================================================
// In-Memory Policy Repository
// ============================================================================

#[derive(Clone, Default)]
pub struct InMemoryPolicyRepository {
    policies: Arc<RwLock<HashMap<String, PolicyModel>>>,
}

impl InMemoryPolicyRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PolicyReader for InMemoryPolicyRepository {
    async fn find_by_vault(&self, vault_address: &str) -> Result<Option<PolicyModel>, ApiError> {
        let read_guard = self.policies.read().await;
        Ok(read_guard.get(vault_address).cloned())
    }

    async fn list_all(&self) -> Result<Vec<PolicyModel>, ApiError> {
        let read_guard = self.policies.read().await;
        Ok(read_guard.values().cloned().collect())
    }
}

#[async_trait]
impl PolicyWriter for InMemoryPolicyRepository {
    async fn upsert(&self, policy: &PolicyModel) -> Result<PolicyModel, ApiError> {
        let mut write_guard = self.policies.write().await;
        let mut p = policy.clone();
        p.updated_at = Utc::now();
        write_guard.insert(policy.vault_address.clone(), p.clone());
        Ok(p)
    }
}

// ============================================================================
// In-Memory Execution Repository
// ============================================================================

#[derive(Clone, Default)]
pub struct InMemoryExecutionRepository {
    executions: Arc<RwLock<Vec<ExecutionModel>>>,
}

impl InMemoryExecutionRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ExecutionReader for InMemoryExecutionRepository {
    async fn list_all(&self) -> Result<Vec<ExecutionModel>, ApiError> {
        let read_guard = self.executions.read().await;
        Ok(read_guard.clone())
    }

    async fn list_by_vault(&self, vault_address: &str) -> Result<Vec<ExecutionModel>, ApiError> {
        let read_guard = self.executions.read().await;
        Ok(read_guard
            .iter()
            .filter(|e| e.vault_address == vault_address)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl ExecutionRecorder for InMemoryExecutionRepository {
    async fn create(&self, execution: &ExecutionModel) -> Result<ExecutionModel, ApiError> {
        let mut write_guard = self.executions.write().await;
        let mut exec = execution.clone();
        if exec.execution_id.is_nil() {
            exec.execution_id = Uuid::new_v4();
        }
        write_guard.push(exec.clone());
        Ok(exec)
    }
}

// ============================================================================
// In-Memory Event Repository
// ============================================================================

#[derive(Clone, Default)]
pub struct InMemoryEventRepository {
    events: Arc<RwLock<Vec<EventModel>>>,
}

impl InMemoryEventRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EventReader for InMemoryEventRepository {
    async fn list_all(&self) -> Result<Vec<EventModel>, ApiError> {
        let read_guard = self.events.read().await;
        Ok(read_guard.clone())
    }

    async fn find_pending(&self) -> Result<Vec<EventModel>, ApiError> {
        let read_guard = self.events.read().await;
        Ok(read_guard
            .iter()
            .filter(|e| e.status == "PENDING" || e.status == "pending")
            .cloned()
            .collect())
    }

    async fn list_by_vault(&self, vault_address: &str) -> Result<Vec<EventModel>, ApiError> {
        let read_guard = self.events.read().await;
        Ok(read_guard
            .iter()
            .filter(|e| e.vault_address.as_deref() == Some(vault_address))
            .cloned()
            .collect())
    }
}

#[async_trait]
impl EventWriter for InMemoryEventRepository {
    async fn create(&self, event: &EventModel) -> Result<EventModel, ApiError> {
        let mut write_guard = self.events.write().await;
        let mut ev = event.clone();
        if ev.event_id.is_nil() {
            ev.event_id = Uuid::new_v4();
        }
        write_guard.push(ev.clone());
        Ok(ev)
    }
}

// ============================================================================
// In-Memory Portfolio Repository
// ============================================================================

#[derive(Clone, Default)]
pub struct InMemoryPortfolioRepository {
    positions: Arc<RwLock<HashMap<(String, String), PortfolioModel>>>,
}

impl InMemoryPortfolioRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PortfolioReader for InMemoryPortfolioRepository {
    async fn list_by_vault(&self, vault_address: &str) -> Result<Vec<PortfolioModel>, ApiError> {
        let read_guard = self.positions.read().await;
        Ok(read_guard
            .values()
            .filter(|p| p.vault_address == vault_address)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl PortfolioWriter for InMemoryPortfolioRepository {
    async fn upsert_position(&self, position: &PortfolioModel) -> Result<PortfolioModel, ApiError> {
        let mut write_guard = self.positions.write().await;
        let key = (position.vault_address.clone(), position.asset_mint.clone());
        let mut pos = position.clone();
        pos.updated_at = Utc::now();
        write_guard.insert(key, pos.clone());
        Ok(pos)
    }
}

// ============================================================================
// In-Memory DBC Pool Repository
// ============================================================================

#[derive(Clone, Default)]
pub struct InMemoryDbcPoolRepository {
    pools: Arc<RwLock<HashMap<String, DbcPoolModel>>>,
}

impl InMemoryDbcPoolRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DbcPoolReader for InMemoryDbcPoolRepository {
    async fn list_all(&self) -> Result<Vec<DbcPoolModel>, ApiError> {
        let read_guard = self.pools.read().await;
        Ok(read_guard.values().cloned().collect())
    }

    async fn find_by_pool_address(&self, address: &str) -> Result<Option<DbcPoolModel>, ApiError> {
        let read_guard = self.pools.read().await;
        Ok(read_guard.get(address).cloned())
    }
}

#[async_trait]
impl DbcPoolWriter for InMemoryDbcPoolRepository {
    async fn create(&self, req: &CreateDbcPoolRequest) -> Result<DbcPoolModel, ApiError> {
        let mut write_guard = self.pools.write().await;
        let now = Utc::now();
        let model = DbcPoolModel {
            pool_address: req.pool_address.clone(),
            config_address: req.config_address.clone(),
            base_mint: req.base_mint.clone(),
            quote_mint: req.quote_mint.clone(),
            token_name: req.token_name.clone(),
            token_symbol: req.token_symbol.clone(),
            tx_signature: req.tx_signature.clone(),
            creator: req.creator.clone(),
            initial_price_usd: req.initial_price_usd,
            current_price_usd: req.current_price_usd,
            curve_progress_pct: req.curve_progress_pct,
            is_migrated: req.is_migrated,
            creation_timestamp: req.creation_timestamp.unwrap_or(now),
            created_at: now,
            updated_at: now,
        };
        write_guard.insert(req.pool_address.clone(), model.clone());
        Ok(model)
    }
}
