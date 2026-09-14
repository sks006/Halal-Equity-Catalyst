//! Shared constants for Equity Catalyst.

/// Maximum basis points representing 100.00%
pub const MAX_BPS: u16 = 10_000;

/// Default stop-loss threshold: 5.00%
pub const DEFAULT_STOP_LOSS_BPS: u16 = 500;

/// Default take-profit threshold: 15.00%
pub const DEFAULT_TAKE_PROFIT_BPS: u16 = 1_500;

/// Default rebalance trigger threshold: 2.00% drift
pub const DEFAULT_REBALANCE_THRESHOLD_BPS: u16 = 200;

/// Minimum safety health factor (1.20 = 12,000 bps)
pub const MIN_HEALTH_FACTOR_BPS: u16 = 12_000;

/// Maximum length for asset names
pub const MAX_ASSET_NAME_LEN: usize = 32;

/// Maximum length for asset ticker symbols
pub const MAX_SYMBOL_LEN: usize = 12;
