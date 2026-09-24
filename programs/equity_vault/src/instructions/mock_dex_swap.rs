use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct MockDexSwap<'info> {
    pub vault: Signer<'info>,

    #[account(mut)]
    pub source_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub destination_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

/// On-chain mock DEX swap handler used exclusively for test execution and CPI validation.
///
/// When invoked via CPI:
/// - If a liquidity reserve account is supplied in `remaining_accounts`, it transfers `amount_out`
///   into `destination_token` to simulate successful swap delivery.
/// - If no liquidity reserve is supplied, no tokens are delivered, triggering `SlippageExceeded`
///   in `execute_action` to prove balance-delta derivation cannot be faked.
pub fn mock_dex_swap<'info>(
    ctx: Context<'_, '_, '_, 'info, MockDexSwap<'info>>,
    amount_in: u64,
    amount_out: u64,
) -> Result<()> {
    msg!(
        "MockDexSwap CPI executed: amount_in={}, amount_out={}",
        amount_in,
        amount_out
    );

    // If a reserve account is provided in remaining_accounts, transfer amount_out to destination
    if let Some(reserve_info) = ctx.remaining_accounts.first() {
        let cpi_accounts = Transfer {
            from: reserve_info.clone(),
            to: ctx.accounts.destination_token.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount_out)?;
    }

    Ok(())
}
