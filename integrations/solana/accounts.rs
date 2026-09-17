use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Equity Catalyst Vault Program ID on Solana Devnet
pub const PROGRAM_ID_STR: &str = "8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH";

pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID_STR).expect("Invalid PROGRAM_ID_STR constant")
}

pub const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const SPL_ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const JUPITER_V6_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
pub const METEORA_DBC_PROGRAM_ID: &str = "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN";

// --- Anchor 8-Byte Account Discriminators ---
pub const VAULT_ACCOUNT_DISCRIMINATOR: [u8; 8] = [211, 8, 232, 43, 2, 152, 117, 119];
pub const POLICY_ACCOUNT_DISCRIMINATOR: [u8; 8] = [222, 135, 7, 163, 235, 177, 33, 68];
pub const USER_SHARES_ACCOUNT_DISCRIMINATOR: [u8; 8] = [148, 201, 81, 76, 109, 139, 152, 190];
pub const POSITION_ACCOUNT_DISCRIMINATOR: [u8; 8] = [170, 188, 143, 228, 122, 64, 247, 208];
pub const LOAN_ACCOUNT_DISCRIMINATOR: [u8; 8] = [20, 195, 70, 117, 165, 227, 182, 1];
pub const EXECUTION_ACCOUNT_DISCRIMINATOR: [u8; 8] = [50, 148, 225, 163, 129, 33, 229, 40];

/// Compute standard Anchor account discriminator: Sha256("account:<AccountName>")[..8]
pub fn compute_account_discriminator(account_name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("account:{}", account_name).as_bytes());
    let result = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&result[..8]);
    disc
}

/// Compute standard Anchor instruction discriminator: Sha256("global:<InstructionName>")[..8]
pub fn compute_instruction_discriminator(instruction_name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{}", instruction_name).as_bytes());
    let result = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&result[..8]);
    disc
}

// --- Account Data Structures ---

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct VaultAccount {
    pub authority: Pubkey,
    pub asset_mint: Pubkey,
    pub policy: Pubkey,
    pub name: String,
    pub symbol: String,
    pub total_deposits: u64,
    pub total_shares: u64,
    pub is_paused: bool,
    pub bump: u8,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct PolicyAccount {
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

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct UserSharesAccount {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub shares: u64,
    pub bump: u8,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct PositionAccount {
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

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct ExecutionAccount {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplTokenAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub is_initialized: bool,
    pub is_frozen: bool,
}

// --- PDA Derivations ---

/// Derives Vault PDA: `seeds = [b"vault", authority.as_ref(), name.as_bytes()]`
pub fn find_vault_pda(authority: &Pubkey, name: &str, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", authority.as_ref(), name.as_bytes()], program_id)
}

/// Derives Policy PDA: `seeds = [b"policy", vault.as_ref()]`
pub fn find_policy_pda(vault: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"policy", vault.as_ref()], program_id)
}

/// Derives UserShares PDA: `seeds = [b"user_shares", vault.as_ref(), user.as_ref()]`
pub fn find_user_shares_pda(vault: &Pubkey, user: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"user_shares", vault.as_ref(), user.as_ref()], program_id)
}

/// Derives Loan PDA: `seeds = [b"loan", vault.as_ref(), borrower.as_ref()]`
pub fn find_loan_pda(vault: &Pubkey, borrower: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"loan", vault.as_ref(), borrower.as_ref()], program_id)
}

/// Derives Position PDA: `seeds = [b"position", vault.as_ref(), asset_mint.as_ref()]`
pub fn find_position_pda(vault: &Pubkey, asset_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"position", vault.as_ref(), asset_mint.as_ref()],
        program_id,
    )
}

/// Derives Execution PDA: `seeds = [b"execution", vault.as_ref(), &execution_id.to_le_bytes()]`
pub fn find_execution_pda(vault: &Pubkey, execution_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"execution", vault.as_ref(), &execution_id.to_le_bytes()],
        program_id,
    )
}

/// Derives Associated Token Account (ATA) address for a given wallet and mint
pub fn find_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let spl_token = Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).unwrap();
    let ata_program = Pubkey::from_str(SPL_ASSOCIATED_TOKEN_PROGRAM_ID).unwrap();
    let (pda, _) = Pubkey::find_program_address(
        &[wallet.as_ref(), spl_token.as_ref(), mint.as_ref()],
        &ata_program,
    );
    pda
}

// --- Account Parsers & Deserializers ---

pub fn parse_anchor_account<T: BorshDeserialize>(
    data: &[u8],
    expected_discriminator: &[u8; 8],
) -> Result<T, crate::SolanaError> {
    if data.len() < 8 {
        return Err(crate::SolanaError::AccountDataTooShort {
            expected: 8,
            actual: data.len(),
        });
    }

    let (disc, body) = data.split_at(8);
    if disc != expected_discriminator {
        return Err(crate::SolanaError::InvalidDiscriminator {
            expected: *expected_discriminator,
            actual: disc.try_into().unwrap_or([0u8; 8]),
        });
    }

    T::try_from_slice(body).map_err(|e| crate::SolanaError::DeserializationFailed(e.to_string()))
}

pub fn parse_vault(data: &[u8]) -> Result<VaultAccount, crate::SolanaError> {
    parse_anchor_account(data, &VAULT_ACCOUNT_DISCRIMINATOR)
}

pub fn parse_policy(data: &[u8]) -> Result<PolicyAccount, crate::SolanaError> {
    parse_anchor_account(data, &POLICY_ACCOUNT_DISCRIMINATOR)
}

pub fn parse_user_shares(data: &[u8]) -> Result<UserSharesAccount, crate::SolanaError> {
    parse_anchor_account(data, &USER_SHARES_ACCOUNT_DISCRIMINATOR)
}

pub fn parse_position(data: &[u8]) -> Result<PositionAccount, crate::SolanaError> {
    parse_anchor_account(data, &POSITION_ACCOUNT_DISCRIMINATOR)
}

pub fn parse_loan(data: &[u8]) -> Result<LoanAccount, crate::SolanaError> {
    parse_anchor_account(data, &LOAN_ACCOUNT_DISCRIMINATOR)
}

pub fn parse_execution(data: &[u8]) -> Result<ExecutionAccount, crate::SolanaError> {
    parse_anchor_account(data, &EXECUTION_ACCOUNT_DISCRIMINATOR)
}

/// Decodes standard 165-byte SPL Token account
pub fn parse_spl_token(data: &[u8]) -> Result<SplTokenAccount, crate::SolanaError> {
    if data.len() < 165 {
        return Err(crate::SolanaError::AccountDataTooShort {
            expected: 165,
            actual: data.len(),
        });
    }

    let mint = Pubkey::try_from(&data[0..32])
        .map_err(|e| crate::SolanaError::DeserializationFailed(e.to_string()))?;
    let owner = Pubkey::try_from(&data[32..64])
        .map_err(|e| crate::SolanaError::DeserializationFailed(e.to_string()))?;
    let amount =
        u64::from_le_bytes(data[64..72].try_into().map_err(|_| {
            crate::SolanaError::DeserializationFailed("Invalid amount bytes".into())
        })?);
    let state = data[108]; // 0 = uninitialized, 1 = initialized, 2 = frozen

    Ok(SplTokenAccount {
        mint,
        owner,
        amount,
        is_initialized: state == 1 || state == 2,
        is_frozen: state == 2,
    })
}

// --- Anchor Program Events ---

/// Compute standard Anchor event discriminator: Sha256("event:<EventName>")[..8]
pub fn compute_event_discriminator(event_name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("event:{}", event_name).as_bytes());
    let result = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&result[..8]);
    disc
}

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct VaultInitializedEvent {
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

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct DepositEvent {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub shares: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct WithdrawEvent {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub shares: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct PolicyUpdatedEvent {
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

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct VaultPauseToggledEvent {
    pub vault: Pubkey,
    pub is_paused: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParsedProgramEvent {
    VaultInitialized(VaultInitializedEvent),
    Deposit(DepositEvent),
    Withdraw(WithdrawEvent),
    PolicyUpdated(PolicyUpdatedEvent),
    VaultPauseToggled(VaultPauseToggledEvent),
}

impl ParsedProgramEvent {
    pub fn vault_address(&self) -> String {
        match self {
            Self::VaultInitialized(e) => e.vault.to_string(),
            Self::Deposit(e) => e.vault.to_string(),
            Self::Withdraw(e) => e.vault.to_string(),
            Self::PolicyUpdated(e) => e.vault.to_string(),
            Self::VaultPauseToggled(e) => e.vault.to_string(),
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::VaultInitialized(_) => "VAULT_INITIALIZED",
            Self::Deposit(_) => "DEPOSIT",
            Self::Withdraw(_) => "WITHDRAW",
            Self::PolicyUpdated(_) => "POLICY_UPDATED",
            Self::VaultPauseToggled(_) => "VAULT_PAUSE_TOGGLED",
        }
    }
}

/// Decodes binary Anchor event payload (8-byte discriminator + Borsh body)
pub fn parse_anchor_event(data: &[u8]) -> Option<ParsedProgramEvent> {
    if data.len() < 8 {
        return None;
    }
    let (disc, body) = data.split_at(8);

    if disc == compute_event_discriminator("Deposit") {
        DepositEvent::try_from_slice(body)
            .ok()
            .map(ParsedProgramEvent::Deposit)
    } else if disc == compute_event_discriminator("Withdraw") {
        WithdrawEvent::try_from_slice(body)
            .ok()
            .map(ParsedProgramEvent::Withdraw)
    } else if disc == compute_event_discriminator("PolicyUpdated") {
        PolicyUpdatedEvent::try_from_slice(body)
            .ok()
            .map(ParsedProgramEvent::PolicyUpdated)
    } else if disc == compute_event_discriminator("VaultPauseToggled") {
        VaultPauseToggledEvent::try_from_slice(body)
            .ok()
            .map(ParsedProgramEvent::VaultPauseToggled)
    } else if disc == compute_event_discriminator("VaultInitialized") {
        VaultInitializedEvent::try_from_slice(body)
            .ok()
            .map(ParsedProgramEvent::VaultInitialized)
    } else {
        None
    }
}

/// Parses an Anchor log line starting with "Program data: <base64>"
pub fn parse_program_data_log(log_line: &str) -> Option<ParsedProgramEvent> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    let trimmed = log_line.trim();
    if let Some(b64_data) = trimmed.strip_prefix("Program data: ") {
        if let Ok(bytes) = BASE64.decode(b64_data.trim()) {
            return parse_anchor_event(&bytes);
        }
    }
    None
}
