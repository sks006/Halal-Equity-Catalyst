use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    hash::hash,
    instruction::{AccountMeta, Instruction},
};

/// Self-program ID acting as on-chain Mock DEX for integration and local test environments.
pub const MOCK_DEX_PROGRAM_ID: Pubkey = pubkey!("8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH");

/// Anchor instruction discriminator: Sha256("global:mock_dex_swap")[..8]
pub fn mock_swap_discriminator() -> [u8; 8] {
    let result = hash(b"global:mock_dex_swap");
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&result.to_bytes()[..8]);
    disc
}

/// Builds an on-chain test/mock CPI swap instruction.
pub fn build_cpi_instruction(
    mock_program_id: &Pubkey,
    vault_authority: &Pubkey,
    vault_input_token: &Pubkey,
    vault_output_token: &Pubkey,
    token_program: &Pubkey,
    remaining_accounts: &[AccountInfo],
    input_amount: u64,
    min_output_amount: u64,
) -> Instruction {
    let mut accounts = Vec::with_capacity(4 + remaining_accounts.len());

    // 1. Vault Authority (Signer via invoke_signed)
    accounts.push(AccountMeta::new_readonly(*vault_authority, true));
    // 2. Vault Input Token Account (source)
    accounts.push(AccountMeta::new(*vault_input_token, false));
    // 3. Vault Output Token Account (destination)
    accounts.push(AccountMeta::new(*vault_output_token, false));
    // 4. Token Program
    accounts.push(AccountMeta::new_readonly(*token_program, false));

    // Optional reserve/liquidity accounts in remaining accounts
    for acc in remaining_accounts {
        if acc.is_writable {
            accounts.push(AccountMeta::new(*acc.key, acc.is_signer));
        } else {
            accounts.push(AccountMeta::new_readonly(*acc.key, acc.is_signer));
        }
    }

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&mock_swap_discriminator());
    data.extend_from_slice(&input_amount.to_le_bytes());
    data.extend_from_slice(&min_output_amount.to_le_bytes());

    Instruction {
        program_id: *mock_program_id,
        accounts,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_swap_discriminator() {
        let disc = mock_swap_discriminator();
        assert_ne!(disc, [0u8; 8]);
    }
}
