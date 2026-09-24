use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    hash::hash,
    instruction::{AccountMeta, Instruction},
};

/// Official Jupiter v6 Swap Aggregator Program ID on Solana Mainnet and Devnet.
pub const JUPITER_V6_PROGRAM_ID: Pubkey = pubkey!("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");

/// Anchor instruction discriminator: Sha256("global:sharedAccountsRoute")[..8]
pub fn shared_accounts_route_discriminator() -> [u8; 8] {
    let result = hash(b"global:sharedAccountsRoute");
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&result.to_bytes()[..8]);
    disc
}

/// Anchor instruction discriminator: Sha256("global:route")[..8]
pub fn route_discriminator() -> [u8; 8] {
    let result = hash(b"global:route");
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&result.to_bytes()[..8]);
    disc
}

/// Builds an official Jupiter v6 CPI swap instruction.
///
/// In standard Jupiter v6 CPI execution:
/// - `user_transfer_authority` is the Vault PDA.
/// - `vault_input_token` is the source token account owned by the vault.
/// - `vault_output_token` is the destination token account owned by the vault.
/// - `remaining_accounts` supply DEX-specific route accounts (pool accounts, tick arrays, etc.).
pub fn build_cpi_instruction(
    jupiter_program_id: &Pubkey,
    vault_authority: &Pubkey,
    vault_input_token: &Pubkey,
    vault_output_token: &Pubkey,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
    token_program: &Pubkey,
    remaining_accounts: &[AccountInfo],
    input_amount: u64,
    min_output_amount: u64,
    custom_route_data: Option<&[u8]>,
) -> Instruction {
    let mut accounts = Vec::with_capacity(7 + remaining_accounts.len());

    // 1. Token Program
    accounts.push(AccountMeta::new_readonly(*token_program, false));
    // 2. User transfer authority (Vault PDA - signer via invoke_signed)
    accounts.push(AccountMeta::new_readonly(*vault_authority, true));
    // 3. User source token account (Vault input ATA)
    accounts.push(AccountMeta::new(*vault_input_token, false));
    // 4. User destination token account (Vault output ATA)
    accounts.push(AccountMeta::new(*vault_output_token, false));
    // 5. Source Mint
    accounts.push(AccountMeta::new_readonly(*input_mint, false));
    // 6. Destination Mint
    accounts.push(AccountMeta::new_readonly(*output_mint, false));

    // Append remaining DEX route accounts
    for acc in remaining_accounts {
        if acc.is_writable {
            accounts.push(AccountMeta::new(*acc.key, acc.is_signer));
        } else {
            accounts.push(AccountMeta::new_readonly(*acc.key, acc.is_signer));
        }
    }

    // Build instruction data payload
    let data = if let Some(route_bytes) = custom_route_data {
        if !route_bytes.is_empty() {
            route_bytes.to_vec()
        } else {
            build_default_route_payload(input_amount, min_output_amount)
        }
    } else {
        build_default_route_payload(input_amount, min_output_amount)
    };

    Instruction {
        program_id: *jupiter_program_id,
        accounts,
        data,
    }
}

/// Constructs default instruction payload with Jupiter v6 `sharedAccountsRoute` discriminator.
fn build_default_route_payload(in_amount: u64, out_amount_threshold: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(32);
    // 8-byte discriminator
    data.extend_from_slice(&shared_accounts_route_discriminator());
    // in_amount: u64
    data.extend_from_slice(&in_amount.to_le_bytes());
    // out_amount_threshold: u64
    data.extend_from_slice(&out_amount_threshold.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jupiter_discriminators() {
        let disc = shared_accounts_route_discriminator();
        assert_ne!(disc, [0u8; 8]);
        let route_disc = route_discriminator();
        assert_ne!(route_disc, [0u8; 8]);
        assert_ne!(disc, route_disc);
    }

    #[test]
    fn test_build_cpi_instruction() {
        let jup = JUPITER_V6_PROGRAM_ID;
        let vault = Pubkey::new_unique();
        let in_tok = Pubkey::new_unique();
        let out_tok = Pubkey::new_unique();
        let in_mint = Pubkey::new_unique();
        let out_mint = Pubkey::new_unique();
        let tok_prog = Pubkey::new_unique();

        let ix = build_cpi_instruction(
            &jup, &vault, &in_tok, &out_tok, &in_mint, &out_mint, &tok_prog, &[], 100_000, 95_000, None,
        );

        assert_eq!(ix.program_id, jup);
        assert_eq!(ix.accounts.len(), 6);
        assert!(ix.accounts[1].is_signer); // Vault authority must sign
        assert_eq!(ix.data[..8], shared_accounts_route_discriminator());
    }
}
