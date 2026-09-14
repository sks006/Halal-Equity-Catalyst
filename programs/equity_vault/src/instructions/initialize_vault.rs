use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::constants::{POLICY_SEED, VAULT_SEED};
use crate::events::VaultInitialized;
use crate::state::{Policy, Vault};
use crate::utils::validation::{validate_risk_limits, validate_vault_params};

#[derive(Accounts)]
#[instruction(name: String, symbol: String, max_ltv_bps: u16, max_position_bps: u16)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + Vault::INIT_SPACE,
        seeds = [VAULT_SEED, authority.key().as_ref(), name.as_bytes()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init,
        payer = authority,
        space = 8 + Policy::INIT_SPACE,
        seeds = [POLICY_SEED, vault.key().as_ref()],
        bump
    )]
    pub policy: Account<'info, Policy>,

    pub asset_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_vault(
    ctx: Context<InitializeVault>,
    name: String,
    symbol: String,
    max_ltv_bps: u16,
    max_position_bps: u16,
) -> Result<()> {
    validate_vault_params(&name, &symbol)?;
    validate_risk_limits(max_ltv_bps, max_position_bps)?;

    let clock = Clock::get()?;
    let now = clock.unix_timestamp;

    let vault = &mut ctx.accounts.vault;
    vault.authority = ctx.accounts.authority.key();
    vault.asset_mint = ctx.accounts.asset_mint.key();
    vault.policy = ctx.accounts.policy.key();
    vault.name = name.clone();
    vault.symbol = symbol.clone();
    vault.total_deposits = 0;
    vault.total_shares = 0;
    vault.is_paused = false;
    vault.bump = ctx.bumps.vault;
    vault.created_at = now;
    vault.updated_at = now;

    let policy = &mut ctx.accounts.policy;
    policy.vault = vault.key();
    policy.authority = ctx.accounts.authority.key();
    policy.max_ltv_bps = max_ltv_bps;
    policy.max_position_bps = max_position_bps;
    policy.stop_loss_bps = 500; // default 5%
    policy.take_profit_bps = 1500; // default 15%
    policy.rebalance_threshold_bps = 200; // default 2%
    policy.is_active = true;
    policy.bump = ctx.bumps.policy;
    policy.updated_at = now;

    emit!(VaultInitialized {
        vault: vault.key(),
        authority: ctx.accounts.authority.key(),
        policy: policy.key(),
        asset_mint: ctx.accounts.asset_mint.key(),
        name,
        symbol,
        max_ltv_bps,
        max_position_bps,
        timestamp: now,
    });

    Ok(())
}
