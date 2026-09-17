use anchor_lang::prelude::*;

use crate::constants::MAX_BPS;
use crate::errors::VaultError;
use crate::events::PolicyUpdated;
use crate::state::{Policy, Vault};
use crate::utils::validation::validate_risk_limits;

#[derive(Accounts)]
pub struct UpdatePolicy<'info> {
    pub authority: Signer<'info>,

    #[account(
        has_one = authority @ VaultError::UnauthorizedKeeper
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        has_one = vault @ VaultError::UnauthorizedKeeper
    )]
    pub policy: Account<'info, Policy>,
}

pub fn update_policy(
    ctx: Context<UpdatePolicy>,
    min_cash_bps: u16,
    max_position_bps: u16,
    stop_loss_bps: u16,
    take_profit_bps: u16,
    rebalance_threshold_bps: u16,
    is_active: bool,
) -> Result<()> {
    validate_risk_limits(min_cash_bps, max_position_bps)?;
    require!(stop_loss_bps <= MAX_BPS, VaultError::InvalidRiskLimit);
    require!(take_profit_bps <= MAX_BPS, VaultError::InvalidRiskLimit);
    require!(
        rebalance_threshold_bps <= MAX_BPS,
        VaultError::InvalidRiskLimit
    );

    let clock = Clock::get()?;
    let now = clock.unix_timestamp;

    let policy = &mut ctx.accounts.policy;
    policy.min_cash_bps = min_cash_bps;
    policy.max_position_bps = max_position_bps;
    policy.stop_loss_bps = stop_loss_bps;
    policy.take_profit_bps = take_profit_bps;
    policy.rebalance_threshold_bps = rebalance_threshold_bps;
    policy.is_active = is_active;
    policy.updated_at = now;

    emit!(PolicyUpdated {
        vault: ctx.accounts.vault.key(),
        policy: policy.key(),
        min_cash_bps,
        max_position_bps,
        stop_loss_bps,
        take_profit_bps,
        rebalance_threshold_bps,
        is_active,
        timestamp: now,
    });

    Ok(())
}
