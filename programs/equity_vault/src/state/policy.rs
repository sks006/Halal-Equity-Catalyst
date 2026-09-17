use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Policy {
    pub vault: Pubkey,
    pub authority: Pubkey,
    pub min_cash_bps: u16,
    pub max_position_bps: u16,
    pub stop_loss_bps: u16,
    pub take_profit_bps: u16,
    pub rebalance_threshold_bps: u16,
    pub is_active: bool,
    pub bump: u8,
    pub updated_at: i64,
}
