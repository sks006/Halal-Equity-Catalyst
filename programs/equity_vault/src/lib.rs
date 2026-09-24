#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

pub mod constants;
pub mod dex;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH");

#[program]
pub mod equity_vault {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        name: String,
        symbol: String,
        min_cash_bps: u16,
        max_position_bps: u16,
    ) -> Result<()> {
        instructions::initialize_vault::initialize_vault(
            ctx,
            name,
            symbol,
            min_cash_bps,
            max_position_bps,
        )
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::deposit(ctx, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, shares_to_burn: u64) -> Result<()> {
        instructions::withdraw::withdraw(ctx, shares_to_burn)
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
        instructions::update_policy::update_policy(
            ctx,
            min_cash_bps,
            max_position_bps,
            stop_loss_bps,
            take_profit_bps,
            rebalance_threshold_bps,
            is_active,
        )
    }

    pub fn emergency_exit(ctx: Context<EmergencyExit>, is_paused: bool) -> Result<()> {
        instructions::emergency_exit::emergency_exit(ctx, is_paused)
    }

    pub fn set_asset_compliance(
        ctx: Context<SetAssetCompliance>,
        status: u8,
        policy_version: [u8; 32],
        evidence_hash: [u8; 32],
        valid_until: i64,
    ) -> Result<()> {
        instructions::set_asset_compliance::set_asset_compliance(
            ctx,
            status,
            policy_version,
            evidence_hash,
            valid_until,
        )
    }

    pub fn execute_action<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecuteAction<'info>>,
        execution_id: u64,
        action_type: u8,
        input_amount: u64,
        min_output_amount: u64,
    ) -> Result<()> {
        instructions::execute_action::execute_action(
            ctx,
            execution_id,
            action_type,
            input_amount,
            min_output_amount,
        )
    }

    pub fn mock_dex_swap<'info>(
        ctx: Context<'_, '_, 'info, 'info, MockDexSwap<'info>>,
        amount_in: u64,
        amount_out: u64,
    ) -> Result<()> {
        instructions::mock_dex_swap::mock_dex_swap(ctx, amount_in, amount_out)
    }
}
