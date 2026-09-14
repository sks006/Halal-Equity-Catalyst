use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Position {
    pub vault: Pubkey,
    pub asset_mint: Pubkey,
    pub amount: u64,
    pub entry_price: u64,
    pub current_value: u64,
    pub is_active: bool,
    pub bump: u8,
    pub created_at: i64,
    pub updated_at: i64,
}
