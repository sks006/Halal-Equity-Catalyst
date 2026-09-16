# Solana Runtime & Transaction Architecture

## Key Platform Characteristics
- **Sealevel**: Parallel smart contract runtime executing non-overlapping account transactions concurrently.
- **Account Model**: All state is stored in explicit accounts owned by programs. Accounts must maintain rent exemption minimum balances.
- **Compute Budget**: Transactions are capped at 200,000 compute units (CU) by default, expandable up to 1,400,000 CU via `ComputeBudgetInstruction::set_compute_unit_limit`.
- **Priority Fees**: Micro-lamports per CU (`ComputeBudgetInstruction::set_compute_unit_price`) added to ensure deterministic inclusion during network congestion.

## Best Practices
- Keep instruction count compact to fit within standard MTU packet sizes (1232 bytes).
- Verify all account ownership, writable flags, and signer status explicitly in smart contract instructions.
