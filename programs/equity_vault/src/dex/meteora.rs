use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    hash::hash,
    instruction::{AccountMeta, Instruction},
};

/// Meteora Dynamic Bonding Curve (DBC) Program ID.
pub const METEORA_DBC_PROGRAM_ID: Pubkey = pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");

/// Anchor instruction discriminator: Sha256("global:swap")[..8]
pub fn swap_discriminator() -> [u8; 8] {
    let result = hash(b"global:swap");
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&result.to_bytes()[..8]);
    disc
}

/// Builds an official Meteora DBC CPI swap instruction.
pub fn build_cpi_instruction(
    meteora_program_id: &Pubkey,
    vault_authority: &Pubkey,
    vault_input_token: &Pubkey,
    vault_output_token: &Pubkey,
    token_program: &Pubkey,
    remaining_accounts: &[AccountInfo],
    input_amount: u64,
    min_output_amount: u64,
) -> Instruction {
    let mut accounts = Vec::with_capacity(4 + remaining_accounts.len());

    // 1. Vault input token account (source)
    accounts.push(AccountMeta::new(*vault_input_token, false));
    // 2. Vault output token account (destination)
    accounts.push(AccountMeta::new(*vault_output_token, false));
    // 3. User authority (Vault PDA - signer via invoke_signed)
    accounts.push(AccountMeta::new_readonly(*vault_authority, true));
    // 4. Token Program
    accounts.push(AccountMeta::new_readonly(*token_program, false));

    // Append pool state accounts from remaining_accounts
    for acc in remaining_accounts {
        if acc.is_writable {
            accounts.push(AccountMeta::new(*acc.key, acc.is_signer));
        } else {
            accounts.push(AccountMeta::new_readonly(*acc.key, acc.is_signer));
        }
    }

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&swap_discriminator());
    data.extend_from_slice(&input_amount.to_le_bytes());
    data.extend_from_slice(&min_output_amount.to_le_bytes());

    Instruction {
        program_id: *meteora_program_id,
        accounts,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meteora_swap_discriminator() {
        let disc = swap_discriminator();
        assert_ne!(disc, [0u8; 8]);
    }

    #[test]
    fn test_build_meteora_instruction() {
        let meteora = METEORA_DBC_PROGRAM_ID;
        let vault = Pubkey::new_unique();
        let in_tok = Pubkey::new_unique();
        let out_tok = Pubkey::new_unique();
        let tok_prog = Pubkey::new_unique();

        let ix = build_cpi_instruction(
            &meteora,
            &vault,
            &in_tok,
            &out_tok,
            &tok_prog,
            &[],
            50_000,
            48_000,
        );
        assert_eq!(ix.program_id, meteora);
        assert_eq!(ix.accounts.len(), 4);
        assert!(ix.accounts[2].is_signer); // Vault authority must sign
        assert_eq!(ix.data[..8], swap_discriminator());
    }
}
