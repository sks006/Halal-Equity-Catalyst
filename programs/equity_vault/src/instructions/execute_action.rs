use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::{COMPLIANCE_SEED, EXECUTION_SEED, VAULT_SEED};
use crate::dex::{self, is_authorized_dex_program};
use crate::errors::VaultError;
use crate::state::{AssetCompliance, Execution, Vault, COMPLIANCE_STATUS_APPROVED};

#[derive(Accounts)]
#[instruction(execution_id: u64, action_type: u8)]
pub struct ExecuteAction<'info> {
    #[account(
        mut,
        constraint = keeper.key() == vault.authority @ VaultError::UnauthorizedKeeper
    )]
    pub keeper: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, vault.authority.as_ref(), vault.name.as_bytes()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init,
        payer = keeper,
        space = 8 + Execution::INIT_SPACE,
        seeds = [EXECUTION_SEED, vault.key().as_ref(), &execution_id.to_le_bytes()],
        bump
    )]
    pub execution: Account<'info, Execution>,

    pub input_mint: Account<'info, Mint>,
    pub output_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = vault_input_token_account.owner == vault.key() @ VaultError::ConstraintViolation,
        constraint = vault_input_token_account.mint == input_mint.key() @ VaultError::InvalidTokenMint,
    )]
    pub vault_input_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault_output_token_account.owner == vault.key() @ VaultError::ConstraintViolation,
        constraint = vault_output_token_account.mint == output_mint.key() @ VaultError::InvalidTokenMint,
    )]
    pub vault_output_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [COMPLIANCE_SEED, compliance.asset_mint.as_ref()],
        bump = compliance.bump,
    )]
    pub compliance: Account<'info, AssetCompliance>,

    /// CHECK: Validated against authorized DEX whitelist in account constraints
    #[account(
        constraint = is_authorized_dex_program(&dex_program.key()) @ VaultError::UnauthorizedDexProgram
    )]
    pub dex_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn execute_action<'info>(
    ctx: Context<'_, '_, 'info, 'info, ExecuteAction<'info>>,
    execution_id: u64,
    action_type: u8,
    input_amount: u64,
    min_output_amount: u64,
) -> Result<()> {
    let vault = &ctx.accounts.vault;
    require!(!vault.is_paused, VaultError::VaultPaused);

    // Spot swap (1) or Emergency exit (4) only. No margin, no lending, no shorting, no derivatives.
    require!(
        action_type == 1 || action_type == 4,
        VaultError::ProhibitedLeverage
    );

    // Validate trade amounts
    require!(input_amount > 0, VaultError::DepositTooSmall);
    require!(min_output_amount > 0, VaultError::InvalidAmount);

    let compliance = &ctx.accounts.compliance;
    let input_key = ctx.accounts.input_mint.key();
    let output_key = ctx.accounts.output_mint.key();

    // Enforce distinct input and output mints
    require!(input_key != output_key, VaultError::InvalidTokenMint);

    // Validate that traded mints match vault asset and compliance approved asset
    let is_buy = input_key == vault.asset_mint && output_key == compliance.asset_mint;
    let is_sell = input_key == compliance.asset_mint && output_key == vault.asset_mint;
    require!(is_buy || is_sell, VaultError::InvalidTokenMint);

    // Enforce that compliance record matches one of the traded mints
    require!(
        compliance.asset_mint == input_key || compliance.asset_mint == output_key,
        VaultError::ComplianceMintMismatch
    );

    // Enforce Shariah compliance status is strictly APPROVED
    require!(
        compliance.status == COMPLIANCE_STATUS_APPROVED,
        VaultError::AssetNotApproved
    );

    // Enforce temporal validity: clock.unix_timestamp < valid_until
    let clock = Clock::get()?;
    let now = clock.unix_timestamp;
    require!(now < compliance.valid_until, VaultError::ComplianceExpired);

    // Ensure vault holds sufficient funds in input token account
    require!(
        ctx.accounts.vault_input_token_account.amount >= input_amount,
        VaultError::InsufficientFunds
    );

    // Snapshot pre-execution output balance
    let pre_output_balance = ctx.accounts.vault_output_token_account.amount;

    // Prepare Vault PDA Signer Seeds
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

    // Execute real DEX Cross-Program Invocation (CPI)
    dex::dispatch_dex_cpi(
        &ctx.accounts.dex_program.to_account_info(),
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.vault_input_token_account.to_account_info(),
        &ctx.accounts.vault_output_token_account.to_account_info(),
        &ctx.accounts.input_mint.to_account_info(),
        &ctx.accounts.output_mint.to_account_info(),
        &ctx.accounts.token_program.to_account_info(),
        ctx.remaining_accounts,
        signer_seeds,
        input_amount,
        min_output_amount,
        None,
    )?;

    // Reload output token account to measure real balance delta
    ctx.accounts.vault_output_token_account.reload()?;
    let post_output_balance = ctx.accounts.vault_output_token_account.amount;

    // Validate that: after_balance >= before_balance
    // unless the transaction explicitly supports negative balance delta semantics.
    require!(
        post_output_balance >= pre_output_balance,
        VaultError::NegativeBalanceDelta
    );

    // Derive actual output strictly from token-account balance deltas (Requirements 6, 7, 8)
    // Never derived from minimum output, expected output, oracle price, or quote estimate.
    let actual_output_amount = post_output_balance
        .checked_sub(pre_output_balance)
        .ok_or(VaultError::MathOverflow)?;

    // Enforce slippage: actual output must meet or exceed minimum output
    require!(
        actual_output_amount >= min_output_amount,
        VaultError::SlippageExceeded
    );

    // Write execution record with verified actual output amount
    let execution = &mut ctx.accounts.execution;
    execution.vault = vault.key();
    execution.execution_id = execution_id;
    execution.action_type = action_type;
    execution.status = 1; // Executed
    execution.input_mint = input_key;
    execution.output_mint = output_key;
    execution.input_amount = input_amount;
    execution.min_output_amount = min_output_amount;
    execution.actual_output_amount = actual_output_amount; // REAL DERIVED BALANCE DELTA
    execution.executed_at = now;
    execution.bump = ctx.bumps.execution;

    // Emit on-chain event capturing token-account balance deltas
    emit!(crate::events::ActionExecuted {
        vault: vault.key(),
        execution_id,
        action_type,
        input_mint: input_key,
        output_mint: output_key,
        requested_input: input_amount,
        minimum_output: min_output_amount,
        actual_output: actual_output_amount,
        before_balance: pre_output_balance,
        after_balance: post_output_balance,
        timestamp: now,
    });

    Ok(())
}
