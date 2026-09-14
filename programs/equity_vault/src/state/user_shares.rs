use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct UserShares {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub shares: u64,
    pub bump: u8,
    pub updated_at: i64,
}
