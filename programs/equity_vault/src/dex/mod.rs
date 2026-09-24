pub mod jupiter;
pub mod meteora;
pub mod mock;

use anchor_lang::prelude::*;
use crate::errors::VaultError;

pub use jupiter::JUPITER_V6_PROGRAM_ID;
pub use meteora::METEORA_DBC_PROGRAM_ID;
pub use mock::MOCK_DEX_PROGRAM_ID;

/// Validates whether a target program ID belongs to the authorized DEX whitelist.
/// Prevents the vault from invoking arbitrary programs (Security Requirement 9).
pub fn is_authorized_dex_program(program_id: &Pubkey) -> bool {
    *program_id == JUPITER_V6_PROGRAM_ID
        || *program_id == METEORA_DBC_PROGRAM_ID
        || *program_id == MOCK_DEX_PROGRAM_ID
}

/// Dispatches an authentic Cross-Program Invocation (CPI) to an authorized DEX.
///
/// Signed with Vault PDA seeds:
/// `[b"vault", authority, name, bump]`
pub fn dispatch_dex_cpi<'info>(
    dex_program: &AccountInfo<'info>,
    vault_authority: &AccountInfo<'info>,
    vault_input_token: &AccountInfo<'info>,
    vault_output_token: &AccountInfo<'info>,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    signer_seeds: &[&[&[u8]]],
    input_amount: u64,
    min_output_amount: u64,
    custom_route_data: Option<&[u8]>,
) -> Result<()> {
    // 1. Strictly fail closed if the DEX program is not in the authorized whitelist
    require!(
        is_authorized_dex_program(dex_program.key),
        VaultError::UnauthorizedDexProgram
    );

    // 2. Build the appropriate DEX CPI instruction
    let ix = if *dex_program.key == JUPITER_V6_PROGRAM_ID {
        jupiter::build_cpi_instruction(
            dex_program.key,
            vault_authority.key,
            vault_input_token.key,
            vault_output_token.key,
            input_mint.key,
            output_mint.key,
            token_program.key,
            remaining_accounts,
            input_amount,
            min_output_amount,
            custom_route_data,
        )
    } else if *dex_program.key == METEORA_DBC_PROGRAM_ID {
        meteora::build_cpi_instruction(
            dex_program.key,
            vault_authority.key,
            vault_input_token.key,
            vault_output_token.key,
            token_program.key,
            remaining_accounts,
            input_amount,
            min_output_amount,
        )
    } else if *dex_program.key == MOCK_DEX_PROGRAM_ID {
        mock::build_cpi_instruction(
            dex_program.key,
            vault_authority.key,
            vault_input_token.key,
            vault_output_token.key,
            token_program.key,
            remaining_accounts,
            input_amount,
            min_output_amount,
        )
    } else {
        return Err(VaultError::UnauthorizedDexProgram.into());
    };

    // 3. Assemble account infos slice required for invoke_signed
    let mut account_infos = Vec::with_capacity(7 + remaining_accounts.len());
    account_infos.push(dex_program.clone());
    account_infos.push(token_program.clone());
    account_infos.push(vault_authority.clone());
    account_infos.push(vault_input_token.clone());
    account_infos.push(vault_output_token.clone());
    account_infos.push(input_mint.clone());
    account_infos.push(output_mint.clone());

    for acc in remaining_accounts {
        account_infos.push(acc.clone());
    }

    // 4. Perform on-chain CPI with Vault PDA seeds
    anchor_lang::solana_program::program::invoke_signed(&ix, &account_infos, signer_seeds).map_err(|e| {
        msg!("DEX CPI invocation failed: {:?}", e);
        VaultError::DexCpiFailed.into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dex_whitelist() {
        assert!(is_authorized_dex_program(&JUPITER_V6_PROGRAM_ID));
        assert!(is_authorized_dex_program(&METEORA_DBC_PROGRAM_ID));
        assert!(is_authorized_dex_program(&MOCK_DEX_PROGRAM_ID));

        let random_program = Pubkey::new_unique();
        assert!(!is_authorized_dex_program(&random_program));

        let system_program = anchor_lang::solana_program::system_program::id();
        assert!(!is_authorized_dex_program(&system_program));
    }
}
