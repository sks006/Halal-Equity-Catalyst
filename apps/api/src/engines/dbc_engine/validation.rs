use crate::engines::dbc_engine::config::DbcConfigRequest;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum DbcValidationError {
    #[error("Asset identifier cannot be empty")]
    EmptyAsset,

    #[error("Unsupported quote token: {0}. Supported quote tokens: USDC, USDT, SOL, WSOL")]
    UnsupportedQuoteToken(String),

    #[error("Initial price must be strictly positive, got {0}")]
    InvalidInitialPrice(f64),

    #[error("Graduation threshold must be at least 10.0 quote units, got {0}")]
    InvalidGraduationThreshold(f64),

    #[error("Total token supply must be strictly positive, got {0}")]
    InvalidTotalSupply(f64),
}

/// Validates all parameters in a DBC configuration request.
pub fn validate_dbc_request(req: &DbcConfigRequest) -> Result<(), DbcValidationError> {
    if req.asset.trim().is_empty() {
        return Err(DbcValidationError::EmptyAsset);
    }

    let quote = req.quote_token.trim().to_uppercase();
    match quote.as_str() {
        "USDC" | "USDT" | "SOL" | "WSOL" => {}
        _ => {
            return Err(DbcValidationError::UnsupportedQuoteToken(
                req.quote_token.clone(),
            ))
        }
    }

    if req.initial_price <= 0.0 || req.initial_price.is_nan() || req.initial_price.is_infinite() {
        return Err(DbcValidationError::InvalidInitialPrice(req.initial_price));
    }

    if req.graduation_threshold < 10.0
        || req.graduation_threshold.is_nan()
        || req.graduation_threshold.is_infinite()
    {
        return Err(DbcValidationError::InvalidGraduationThreshold(
            req.graduation_threshold,
        ));
    }

    if let Some(supply) = req.total_supply {
        if supply <= 0.0 || supply.is_nan() || supply.is_infinite() {
            return Err(DbcValidationError::InvalidTotalSupply(supply));
        }
    }

    Ok(())
}

/// Returns the decimals and known mint address for supported quote tokens.
pub fn resolve_quote_token_info(quote_token: &str) -> (u8, &'static str) {
    match quote_token.trim().to_uppercase().as_str() {
        "USDC" => (6, crate::engines::dbc_engine::registry::MAINNET_USDC_MINT),
        "DEVNET_USDC" => (6, crate::engines::dbc_engine::registry::DEVNET_USDC_MINT),
        "USDT" => (6, "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"),
        "SOL" | "WSOL" => (9, crate::engines::dbc_engine::registry::WRAPPED_SOL_MINT),
        _ => (6, crate::engines::dbc_engine::registry::MAINNET_USDC_MINT),
    }
}
