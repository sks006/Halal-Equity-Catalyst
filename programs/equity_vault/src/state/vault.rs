use crate::constants::{MAX_NAME_LEN, MAX_SYMBOL_LEN};
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub authority: Pubkey,
    pub asset_mint: Pubkey,
    pub policy: Pubkey,
    #[max_len(MAX_NAME_LEN)]
    pub name: String,
    #[max_len(MAX_SYMBOL_LEN)]
    pub symbol: String,
    pub total_deposits: u64,
    pub total_shares: u64,
    pub is_paused: bool,
    pub bump: u8,
    pub created_at: i64,
    pub updated_at: i64,
}
