use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Execution {
    pub vault: Pubkey,
    pub execution_id: u64,
    pub action_type: u8,
    pub status: u8,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub input_amount: u64,
    pub min_output_amount: u64,
    pub actual_output_amount: u64,
    pub executed_at: i64,
    pub bump: u8,
}
