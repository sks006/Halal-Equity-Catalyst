use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{USER_SHARES_SEED, VAULT_SEED};
use crate::errors::VaultError;
use crate::events::Withdraw as WithdrawEvent;
use crate::state::{UserShares, Vault};
use crate::utils::math::calculate_assets_to_withdraw;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        has_one = asset_mint @ VaultError::InvalidTokenMint
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        seeds = [USER_SHARES_SEED, vault.key().as_ref(), user.key().as_ref()],
        bump = user_shares.bump,
        constraint = user_shares.user == user.key() @ VaultError::UnauthorizedKeeper
    )]
    pub user_shares: Account<'info, UserShares>,

    #[account(
        mut,
        constraint = user_asset_account.mint == asset_mint.key() @ VaultError::InvalidTokenMint,
        constraint = user_asset_account.owner == user.key() @ VaultError::InvalidTokenMint
    )]
    pub user_asset_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault_asset_account.mint == asset_mint.key() @ VaultError::InvalidTokenMint,
        constraint = vault_asset_account.owner == vault.key() @ VaultError::InvalidTokenMint
    )]
    pub vault_asset_account: Account<'info, TokenAccount>,

    pub asset_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

pub fn withdraw(ctx: Context<Withdraw>, shares_to_burn: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    require!(!vault.is_paused, VaultError::VaultPaused);
    require!(shares_to_burn > 0, VaultError::InsufficientShares);

    let user_shares = &mut ctx.accounts.user_shares;
    require!(
        user_shares.shares >= shares_to_burn,
        VaultError::InsufficientShares
    );

    let assets_to_return =
        calculate_assets_to_withdraw(shares_to_burn, vault.total_deposits, vault.total_shares)?;

    // CPI Transfer from Vault token account to User signed by Vault PDA
    let authority_key = vault.authority;
    let vault_name = vault.name.clone();
    let vault_bump = vault.bump;
    let seeds = &[
        VAULT_SEED,
        authority_key.as_ref(),
        vault_name.as_bytes(),
        &[vault_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_asset_account.to_account_info(),
        to: ctx.accounts.user_asset_account.to_account_info(),
        authority: vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token::transfer(cpi_ctx, assets_to_return)?;

    let clock = Clock::get()?;
    let now = clock.unix_timestamp;

    // Update user shares
    user_shares.shares = user_shares
        .shares
        .checked_sub(shares_to_burn)
        .ok_or(VaultError::MathOverflow)?;
    user_shares.updated_at = now;

    // Update vault state
    vault.total_shares = vault
        .total_shares
        .checked_sub(shares_to_burn)
        .ok_or(VaultError::MathOverflow)?;
    vault.total_deposits = vault
        .total_deposits
        .checked_sub(assets_to_return)
        .ok_or(VaultError::MathOverflow)?;
    vault.updated_at = now;

    emit!(WithdrawEvent {
        vault: vault.key(),
        user: ctx.accounts.user.key(),
        amount: assets_to_return,
        shares: shares_to_burn,
        timestamp: now,
    });

    Ok(())
}
