use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Calculation resulted in arithmetic overflow")]
    MathOverflow,
    #[msg("Vault is currently paused")]
    VaultPaused,
    #[msg("Deposit amount is below protocol minimum")]
    DepositTooSmall,
    #[msg("Insufficient vault shares for withdrawal")]
    InsufficientShares,
    #[msg("Cash reserve is below minimum required policy limit")]
    CashReserveTooLow,
    #[msg("Leverage, borrowing, and short selling are strictly prohibited")]
    ProhibitedLeverage,
    #[msg("Oracle price feed is stale or invalid")]
    StaleOraclePrice,
    #[msg("Slippage exceeded the maximum specified limit")]
    SlippageExceeded,
    #[msg("Unauthorized keeper or invalid authority signature")]
    UnauthorizedKeeper,
    #[msg("Policy is currently disabled or in cooldown")]
    PolicyInactive,
    #[msg("Position capacity limit reached for this vault")]
    PositionLimitReached,
    #[msg("Position not found or inactive")]
    PositionNotFound,
    #[msg("Invalid token mint or account mismatch")]
    InvalidTokenMint,
    #[msg("Risk limits exceed maximum allowed basis points (10000 bps)")]
    InvalidRiskLimit,
    #[msg("Vault name exceeds maximum allowed length")]
    NameTooLong,
    #[msg("Vault symbol exceeds maximum allowed length")]
    SymbolTooLong,
}
