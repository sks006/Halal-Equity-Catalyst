//! Constants for on-chain equity vault program.

pub const VAULT_SEED: &[u8] = b"vault";
pub const POSITION_SEED: &[u8] = b"position";
pub const POLICY_SEED: &[u8] = b"policy";
pub const LOAN_SEED: &[u8] = b"loan";
pub const EXECUTION_SEED: &[u8] = b"execution";
pub const USER_SHARES_SEED: &[u8] = b"user_shares";

pub const MAX_NAME_LEN: usize = 32;
pub const MAX_SYMBOL_LEN: usize = 12;
pub const MAX_URI_LEN: usize = 128;

pub const MAX_BPS: u16 = 10_000;
pub const MIN_DEPOSIT_AMOUNT: u64 = 1_000;
