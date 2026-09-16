//! Domain validation primitives for Equity Catalyst.

use std::collections::HashSet;
use thiserror::Error;

use crate::constants::MAX_BPS;
use crate::types::{AssetWeight, BasisPoints, RiskLimits};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Basis points value {0} exceeds maximum allowed 10,000 (100.00%)")]
    InvalidBasisPoints(u16),

    #[error("Portfolio weights sum to {actual} bps, but must sum to {expected} bps (100.00%)")]
    WeightsDoNotSumTo100Percent { actual: u16, expected: u16 },

    #[error("Loan-to-value ratio {actual} bps exceeds maximum policy limit {limit} bps")]
    ExceedsMaxLtv { actual: u16, limit: u16 },

    #[error("Position exposure {actual} bps exceeds maximum policy limit {limit} bps")]
    ExceedsMaxPosition { actual: u16, limit: u16 },

    #[error("Slippage {actual} bps exceeds maximum limit {limit} bps")]
    ExceedsMaxSlippage { actual: u16, limit: u16 },

    #[error("Allocation target cannot have empty weights")]
    EmptyAllocationWeights,

    #[error("Duplicate asset symbol in weights: {0}")]
    DuplicateAssetSymbol(String),

    #[error("Calculation resulted in arithmetic overflow")]
    ArithmeticOverflow,

    #[error("Zero total portfolio value cannot be rebalanced")]
    ZeroPortfolioValue,

    #[error("Invalid mint address: {0}")]
    InvalidMintAddress(String),

    #[error("Invalid vault name: {0}")]
    InvalidVaultName(String),

    #[error("Invalid vault symbol: {0}")]
    InvalidVaultSymbol(String),

    #[error("Asset symbol cannot be empty")]
    EmptySymbol,

    #[error("Invalid asset ID: {0}")]
    InvalidAssetId(String),

    #[error("Missing price feed for asset: {0}")]
    MissingFeed(String),

    #[error("Unsupported asset provider: {0}")]
    UnsupportedProvider(String),

    #[error("Unknown asset: {0}")]
    UnknownAsset(String),

    #[error("Duplicate asset in registry: {0}")]
    DuplicateAsset(String),

    #[error("Invalid price: {0}")]
    InvalidPrice(String),

    #[error("Invalid parameter: {0}")]
    InvalidParam(String),
}

pub fn validate_bps(bps: u16) -> Result<BasisPoints, ValidationError> {
    if bps > MAX_BPS {
        return Err(ValidationError::InvalidBasisPoints(bps));
    }
    Ok(BasisPoints(bps))
}

pub fn validate_weights_sum(weights: &[AssetWeight]) -> Result<(), ValidationError> {
    if weights.is_empty() {
        return Err(ValidationError::EmptyAllocationWeights);
    }

    let mut seen = HashSet::new();
    let mut total: u16 = 0;

    for w in weights {
        if !seen.insert(&w.symbol) {
            return Err(ValidationError::DuplicateAssetSymbol(w.symbol.clone()));
        }
        total = total
            .checked_add(w.target_weight.0)
            .ok_or(ValidationError::ArithmeticOverflow)?;
    }

    if total != MAX_BPS {
        return Err(ValidationError::WeightsDoNotSumTo100Percent {
            actual: total,
            expected: MAX_BPS,
        });
    }

    Ok(())
}

pub fn validate_risk_limits(limits: &RiskLimits) -> Result<(), ValidationError> {
    validate_bps(limits.max_ltv_bps.0)?;
    validate_bps(limits.max_position_bps.0)?;
    validate_bps(limits.max_slippage_bps.0)?;
    Ok(())
}

/// Validates that a string is a well-formed Solana base58 mint address (length 32 to 44 characters, base58 alphabet)
pub fn validate_mint_address(mint: &str) -> Result<(), ValidationError> {
    let len = mint.trim().len();
    if !(32..=44).contains(&len) {
        return Err(ValidationError::InvalidMintAddress(format!(
            "Address length must be between 32 and 44 characters, got {}",
            len
        )));
    }
    // Base58 characters: 1-9, A-H, J-N, P-Z, a-k, m-z (no 0, O, I, l)
    for c in mint.chars() {
        if !c.is_ascii_alphanumeric() || c == '0' || c == 'O' || c == 'I' || c == 'l' {
            return Err(ValidationError::InvalidMintAddress(format!(
                "Invalid base58 character '{}'",
                c
            )));
        }
    }
    Ok(())
}

/// Validates vault name (1..=32 chars) and symbol (1..=12 chars)
pub fn validate_name_and_symbol(name: &str, symbol: &str) -> Result<(), ValidationError> {
    let name_trimmed = name.trim();
    if name_trimmed.is_empty() || name_trimmed.len() > 32 {
        return Err(ValidationError::InvalidVaultName(format!(
            "Vault name length must be between 1 and 32 characters, got {}",
            name_trimmed.len()
        )));
    }

    let symbol_trimmed = symbol.trim();
    if symbol_trimmed.is_empty() || symbol_trimmed.len() > 12 {
        return Err(ValidationError::InvalidVaultSymbol(format!(
            "Vault symbol length must be between 1 and 12 characters, got {}",
            symbol_trimmed.len()
        )));
    }

    Ok(())
}
