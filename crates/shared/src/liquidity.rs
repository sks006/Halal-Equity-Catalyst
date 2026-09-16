//! Canonical Liquidity Provider domain models for bonding curves and AMMs.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::validation::ValidationError;

/// Direction of a liquidity trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeDirection {
    /// Buying base token using quote token (USDC -> Tokenized Stock)
    Buy,
    /// Selling base token for quote token (Tokenized Stock -> USDC)
    Sell,
}

impl fmt::Display for TradeDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buy => write!(f, "buy"),
            Self::Sell => write!(f, "sell"),
        }
    }
}

/// Request for a liquidity swap quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiquidityQuoteRequest {
    pub pool_address: String,
    pub input_mint: String,
    pub output_mint: String,
    pub amount_in: u64,
    pub slippage_bps: u16,
    pub direction: TradeDirection,
}

impl LiquidityQuoteRequest {
    pub fn new(
        pool_address: impl Into<String>,
        input_mint: impl Into<String>,
        output_mint: impl Into<String>,
        amount_in: u64,
        slippage_bps: u16,
        direction: TradeDirection,
    ) -> Result<Self, ValidationError> {
        let pool = pool_address.into();
        let in_m = input_mint.into();
        let out_m = output_mint.into();

        if pool.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "pool_address cannot be empty".into(),
            ));
        }
        if in_m.trim().is_empty() || out_m.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "mint addresses cannot be empty".into(),
            ));
        }
        if in_m == out_m {
            return Err(ValidationError::InvalidParam(
                "input and output mints cannot be identical".into(),
            ));
        }
        if amount_in == 0 {
            return Err(ValidationError::InvalidParam(
                "amount_in must be greater than zero".into(),
            ));
        }
        if slippage_bps > 10_000 {
            return Err(ValidationError::InvalidBasisPoints(slippage_bps));
        }

        Ok(Self {
            pool_address: pool,
            input_mint: in_m,
            output_mint: out_m,
            amount_in,
            slippage_bps,
            direction,
        })
    }
}

/// Evaluated liquidity quote response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidityQuoteResponse {
    pub pool_address: String,
    pub amount_in: u64,
    pub expected_amount_out: u64,
    pub min_amount_out: u64,
    pub price_impact_bps: u16,
    pub fee_amount: u64,
    pub current_price_usd: f64,
    pub effective_execution_price_usd: f64,
}

/// On-chain pool reserves and curve state summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolLiquidityState {
    pub pool_address: String,
    pub config_address: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub base_reserve: u64,
    pub quote_reserve: u64,
    pub sqrt_price_q64: u128,
    pub current_price_usd: f64,
    pub curve_progress_pct: f64,
    pub is_migrated: bool,
}

impl PoolLiquidityState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool_address: impl Into<String>,
        config_address: impl Into<String>,
        base_mint: impl Into<String>,
        quote_mint: impl Into<String>,
        base_reserve: u64,
        quote_reserve: u64,
        sqrt_price_q64: u128,
        current_price_usd: f64,
        curve_progress_pct: f64,
        is_migrated: bool,
    ) -> Result<Self, ValidationError> {
        let pool = pool_address.into();
        if pool.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "pool_address cannot be empty".into(),
            ));
        }
        if current_price_usd <= 0.0 {
            return Err(ValidationError::InvalidPrice(format!(
                "current_price_usd must be positive, got {}",
                current_price_usd
            )));
        }

        Ok(Self {
            pool_address: pool,
            config_address: config_address.into(),
            base_mint: base_mint.into(),
            quote_mint: quote_mint.into(),
            base_reserve,
            quote_reserve,
            sqrt_price_q64,
            current_price_usd,
            curve_progress_pct,
            is_migrated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_liquidity_quote_request() {
        let req = LiquidityQuoteRequest::new(
            "MeteoraPool11111111111111111111111111111111",
            "USDC1111111111111111111111111111111111111111",
            "NVDA1111111111111111111111111111111111111111",
            100_000_000,
            100, // 1%
            TradeDirection::Buy,
        )
        .expect("Valid request");

        assert_eq!(req.direction, TradeDirection::Buy);
        assert_eq!(req.amount_in, 100_000_000);
        assert_eq!(req.slippage_bps, 100);
    }

    #[test]
    fn test_invalid_liquidity_quote_request() {
        // Zero amount
        assert!(
            LiquidityQuoteRequest::new("Pool1", "In1", "Out1", 0, 100, TradeDirection::Buy)
                .is_err()
        );

        // Identical mints
        assert!(
            LiquidityQuoteRequest::new("Pool1", "In1", "In1", 1000, 100, TradeDirection::Buy)
                .is_err()
        );

        // Slippage > 10,000 bps
        assert!(LiquidityQuoteRequest::new(
            "Pool1",
            "In1",
            "Out1",
            1000,
            10_001,
            TradeDirection::Buy
        )
        .is_err());
    }
}
