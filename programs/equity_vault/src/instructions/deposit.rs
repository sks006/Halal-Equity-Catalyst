use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{MIN_DEPOSIT_AMOUNT, USER_SHARES_SEED};
use crate::errors::VaultError;
use crate::events::Deposit as DepositEvent;
use crate::state::{UserShares, Vault};
use crate::utils::math::calculate_shares_to_mint;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        has_one = asset_mint @ VaultError::InvalidTokenMint
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserShares::INIT_SPACE,
        seeds = [USER_SHARES_SEED, vault.key().as_ref(), user.key().as_ref()],
        bump
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
    pub system_program: Program<'info, System>,
}

pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    require!(!vault.is_paused, VaultError::VaultPaused);
    require!(amount >= MIN_DEPOSIT_AMOUNT, VaultError::DepositTooSmall);

    let shares_to_mint =
        calculate_shares_to_mint(amount, vault.total_deposits, vault.total_shares)?;

    // Transfer tokens from user to vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_asset_account.to_account_info(),
        to: ctx.accounts.vault_asset_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    let clock = Clock::get()?;
    let now = clock.unix_timestamp;

    // Update user shares
    let user_shares = &mut ctx.accounts.user_shares;
    user_shares.vault = vault.key();
    user_shares.user = ctx.accounts.user.key();
    user_shares.shares = user_shares
        .shares
        .checked_add(shares_to_mint)
        .ok_or(VaultError::MathOverflow)?;
    user_shares.bump = ctx.bumps.user_shares;
    user_shares.updated_at = now;

    // Update vault state
    vault.total_deposits = vault
        .total_deposits
        .checked_add(amount)
        .ok_or(VaultError::MathOverflow)?;
    vault.total_shares = vault
        .total_shares
        .checked_add(shares_to_mint)
        .ok_or(VaultError::MathOverflow)?;
    vault.updated_at = now;

    emit!(DepositEvent {
        vault: vault.key(),
        user: ctx.accounts.user.key(),
        amount,
        shares: shares_to_mint,
        timestamp: now,
    });

    Ok(())
}
