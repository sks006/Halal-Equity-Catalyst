//! Vault business logic service managing vault lifecycle, deposits, and withdrawals.

use crate::{error::ApiError, models::VaultModel, repositories::VaultRepository};

#[derive(Clone, Debug)]
pub struct VaultService {
    repo: VaultRepository,
}

impl VaultService {
    pub fn new(repo: VaultRepository) -> Self {
        Self { repo }
    }

    /// Creates a new vault record in the repository.
    pub async fn create_vault(&self, vault: &VaultModel) -> Result<VaultModel, ApiError> {
        self.repo.create(vault).await
    }

    /// Retrieves an existing vault by its Solana base58 address.
    pub async fn get_vault(&self, address: &str) -> Result<VaultModel, ApiError> {
        self.repo
            .find_by_address(address)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Vault not found: {}", address)))
    }

    /// Lists all vaults.
    pub async fn list_vaults(&self) -> Result<Vec<VaultModel>, ApiError> {
        self.repo.list_all().await
    }

    /// Synchronizes on-chain vault state to database mirror.
    pub async fn sync_vault_state(
        &self,
        address: &str,
        total_shares: u64,
        total_deposits: u64,
        is_paused: bool,
    ) -> Result<(), ApiError> {
        self.repo
            .update_totals(address, total_shares, total_deposits)
            .await?;
        self.repo.set_paused(address, is_paused).await?;
        Ok(())
    }

    /// Records a deposit into the vault and updates totals.
    pub async fn track_deposit(
        &self,
        address: &str,
        amount: u64,
        shares_minted: u64,
    ) -> Result<VaultModel, ApiError> {
        let vault = self.get_vault(address).await?;

        if vault.is_paused {
            return Err(ApiError::BadRequest(format!(
                "Vault {} is paused, deposits blocked",
                address
            )));
        }

        let new_deposits = vault
            .total_deposits
            .checked_add(amount)
            .ok_or_else(|| ApiError::BadRequest("Deposit amount overflow".to_string()))?;
        let new_shares = vault
            .total_shares
            .checked_add(shares_minted)
            .ok_or_else(|| ApiError::BadRequest("Share minting overflow".to_string()))?;

        self.repo
            .update_totals(address, new_shares, new_deposits)
            .await?;

        self.get_vault(address).await
    }

    /// Records a withdrawal from the vault and updates totals.
    pub async fn track_withdrawal(
        &self,
        address: &str,
        shares_burned: u64,
        amount_returned: u64,
    ) -> Result<VaultModel, ApiError> {
        let vault = self.get_vault(address).await?;

        if vault.is_paused {
            return Err(ApiError::BadRequest(format!(
                "Vault {} is paused, withdrawals blocked",
                address
            )));
        }

        if shares_burned > vault.total_shares {
            return Err(ApiError::BadRequest(format!(
                "Cannot burn {} shares, vault only has {}",
                shares_burned, vault.total_shares
            )));
        }

        if amount_returned > vault.total_deposits {
            return Err(ApiError::BadRequest(format!(
                "Cannot withdraw {} units, vault only has {}",
                amount_returned, vault.total_deposits
            )));
        }

        let new_deposits = vault.total_deposits - amount_returned;
        let new_shares = vault.total_shares - shares_burned;

        self.repo
            .update_totals(address, new_shares, new_deposits)
            .await?;

        self.get_vault(address).await
    }

    /// Toggles the emergency pause state of the vault.
    pub async fn set_paused(&self, address: &str, paused: bool) -> Result<(), ApiError> {
        let _ = self.get_vault(address).await?;
        self.repo.set_paused(address, paused).await
    }
}
