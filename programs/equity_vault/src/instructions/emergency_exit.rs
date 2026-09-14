use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::events::VaultPauseToggled;
use crate::state::Vault;

#[derive(Accounts)]
pub struct EmergencyExit<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ VaultError::UnauthorizedKeeper
    )]
    pub vault: Account<'info, Vault>,
}

pub fn emergency_exit(ctx: Context<EmergencyExit>, is_paused: bool) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.is_paused = is_paused;

    let clock = Clock::get()?;
    let now = clock.unix_timestamp;
    vault.updated_at = now;

    emit!(VaultPauseToggled {
        vault: vault.key(),
        is_paused,
        timestamp: now,
    });

    Ok(())
}
