use crate::constants::MAX_BPS;
use crate::errors::VaultError;
use anchor_lang::prelude::*;

/// Safe calculation of basis points: (amount * bps) / 10_000
pub fn compute_bps(amount: u64, bps: u16) -> Result<u64> {
    require!(bps <= MAX_BPS, VaultError::InvalidRiskLimit);
    let result = (amount as u128)
        .checked_mul(bps as u128)
        .ok_or(VaultError::MathOverflow)?
        .checked_div(MAX_BPS as u128)
        .ok_or(VaultError::MathOverflow)?;
    u64::try_from(result).map_err(|_| VaultError::MathOverflow.into())
}

/// Calculate shares to mint for a given deposit amount.
/// If vault has no existing deposits or shares, 1:1 ratio is used.
/// Otherwise: shares = (amount * total_shares) / total_deposits
pub fn calculate_shares_to_mint(
    amount: u64,
    total_deposits: u64,
    total_shares: u64,
) -> Result<u64> {
    if total_shares == 0 || total_deposits == 0 {
        return Ok(amount);
    }
    let shares = (amount as u128)
        .checked_mul(total_shares as u128)
        .ok_or(VaultError::MathOverflow)?
        .checked_div(total_deposits as u128)
        .ok_or(VaultError::MathOverflow)?;
    u64::try_from(shares).map_err(|_| VaultError::MathOverflow.into())
}

/// Calculate assets to return when redeeming shares.
/// assets = (shares * total_deposits) / total_shares
pub fn calculate_assets_to_withdraw(
    shares: u64,
    total_deposits: u64,
    total_shares: u64,
) -> Result<u64> {
    require!(total_shares > 0, VaultError::InsufficientShares);
    let assets = (shares as u128)
        .checked_mul(total_deposits as u128)
        .ok_or(VaultError::MathOverflow)?
        .checked_div(total_shares as u128)
        .ok_or(VaultError::MathOverflow)?;
    u64::try_from(assets).map_err(|_| VaultError::MathOverflow.into())
}
