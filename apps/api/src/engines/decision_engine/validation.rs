//! Decision pre-flight validation rules.

use super::decision::ExecutionRequest;
use crate::models::{PolicyModel, VaultModel};

/// Performs strict pre-flight validation on the generated execution request.
pub fn validate_decision_preflight(
    vault: &VaultModel,
    policy: &PolicyModel,
    request: &ExecutionRequest,
) -> Result<(), String> {
    if vault.is_paused {
        return Err(format!(
            "Pre-flight failed: Vault {} is paused",
            vault.vault_address
        ));
    }

    if !policy.is_active {
        return Err(format!(
            "Pre-flight failed: Policy for vault {} is marked inactive",
            vault.vault_address
        ));
    }

    if policy.vault_address != vault.vault_address {
        return Err(format!(
            "Pre-flight failed: Policy vault mismatch (expected {}, got {})",
            vault.vault_address, policy.vault_address
        ));
    }

    if request.approved && request.trades.is_empty() && request.action != "NOOP" {
        return Err(
            "Pre-flight failed: Approved decision with non-NOOP action has no trades".to_string(),
        );
    }

    Ok(())
}
