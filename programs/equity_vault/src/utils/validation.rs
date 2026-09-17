use crate::constants::{MAX_BPS, MAX_NAME_LEN, MAX_SYMBOL_LEN};
use crate::errors::VaultError;
use anchor_lang::prelude::*;

pub fn validate_vault_params(name: &str, symbol: &str) -> Result<()> {
    require!(name.len() <= MAX_NAME_LEN, VaultError::NameTooLong);
    require!(!name.is_empty(), VaultError::NameTooLong);
    require!(symbol.len() <= MAX_SYMBOL_LEN, VaultError::SymbolTooLong);
    require!(!symbol.is_empty(), VaultError::SymbolTooLong);
    Ok(())
}

pub fn validate_risk_limits(min_cash_bps: u16, max_position_bps: u16) -> Result<()> {
    require!(min_cash_bps <= MAX_BPS, VaultError::InvalidRiskLimit);
    require!(max_position_bps <= MAX_BPS, VaultError::InvalidRiskLimit);
    Ok(())
}
