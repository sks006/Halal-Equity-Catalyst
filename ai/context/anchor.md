# Anchor Framework Guide & Constraints

## Program Structure (`programs/equity_vault`)
Anchor provides an opinionated framework for Solana programs, automating discriminator generation, Borsh serialization, and account constraint validation.

## Essential Macros & Constraints
- `#[account]`: Marks an account data struct and prepends an 8-byte SHA256 discriminator.
- `#[derive(Accounts)]`: Defines the account context expected by an instruction.
- `has_one = authority @ ErrorCode::Unauthorized`: Enforces that the account's authority field matches the signing authority.
- `seeds = [...], bump`: Derives and verifies a Program Derived Address (PDA).
- `close = recipient`: Closes an account and refunds remaining lamports to the specified recipient.

## Reentrancy Prevention
Always perform balance mutations and state writes *before* invoking Cross-Program Invocations (CPI) to the SPL Token or Meteora programs.
