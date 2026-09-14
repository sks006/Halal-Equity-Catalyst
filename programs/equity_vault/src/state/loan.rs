use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Loan {
    pub vault: Pubkey,
    pub borrower: Pubkey,
    pub collateral_mint: Pubkey,
    pub collateral_amount: u64,
    pub borrowed_amount: u64,
    pub ltv_bps: u16,
    pub interest_rate_bps: u16,
    pub is_active: bool,
    pub bump: u8,
    pub created_at: i64,
    pub updated_at: i64,
}
