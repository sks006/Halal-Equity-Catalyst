use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::constants::{COMPLIANCE_SEED, EXECUTION_SEED};
use crate::errors::VaultError;
use crate::state::{AssetCompliance, Execution, Vault, COMPLIANCE_STATUS_APPROVED};

#[derive(Accounts)]
#[instruction(execution_id: u64, action_type: u8)]
pub struct ExecuteAction<'info> {
    #[account(mut)]
    pub keeper: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", vault.authority.as_ref(), vault.name.as_bytes()],
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
        seeds = [COMPLIANCE_SEED, compliance.asset_mint.as_ref()],
        bump = compliance.bump,
    )]
    pub compliance: Account<'info, AssetCompliance>,

    pub system_program: Program<'info, System>,
}

pub fn execute_action(
    ctx: Context<ExecuteAction>,
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

    let compliance = &ctx.accounts.compliance;
    let input_key = ctx.accounts.input_mint.key();
    let output_key = ctx.accounts.output_mint.key();

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

    let execution = &mut ctx.accounts.execution;
    execution.vault = vault.key();
    execution.execution_id = execution_id;
    execution.action_type = action_type;
    execution.status = 1; // Executed
    execution.input_mint = input_key;
    execution.output_mint = output_key;
    execution.input_amount = input_amount;
    execution.min_output_amount = min_output_amount;
    execution.actual_output_amount = min_output_amount;
    execution.executed_at = now;
    execution.bump = ctx.bumps.execution;

    Ok(())
}
