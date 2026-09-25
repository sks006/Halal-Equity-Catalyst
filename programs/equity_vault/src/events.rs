use anchor_lang::prelude::*;

#[event]
pub struct VaultInitialized {
    pub vault: Pubkey,
    pub authority: Pubkey,
    pub policy: Pubkey,
    pub asset_mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub min_cash_bps: u16,
    pub max_position_bps: u16,
    pub timestamp: i64,
}

#[event]
pub struct Deposit {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub shares: u64,
    pub timestamp: i64,
}

#[event]
pub struct Withdraw {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub shares: u64,
    pub timestamp: i64,
}

#[event]
pub struct PolicyUpdated {
    pub vault: Pubkey,
    pub policy: Pubkey,
    pub min_cash_bps: u16,
    pub max_position_bps: u16,
    pub stop_loss_bps: u16,
    pub take_profit_bps: u16,
    pub rebalance_threshold_bps: u16,
    pub is_active: bool,
    pub timestamp: i64,
}

#[event]
pub struct VaultPauseToggled {
    pub vault: Pubkey,
    pub is_paused: bool,
    pub timestamp: i64,
}

#[event]
pub struct ActionExecuted {
    pub vault: Pubkey,
    pub execution_id: u64,
    pub action_type: u8,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub requested_input: u64,
    pub minimum_output: u64,
    pub actual_output: u64,
    pub before_balance: u64,
    pub after_balance: u64,
    pub timestamp: i64,
}
