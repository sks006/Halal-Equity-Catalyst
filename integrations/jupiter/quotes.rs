//! Quote calculations, slippage validation, and price impact utilities.

use equity_catalyst_shared::types::BasisPoints;
use tracing::debug;

use crate::types::{JupiterError, QuoteResponse};

/// Converts Jupiter's `priceImpactPct` string (e.g. "0.05" for 0.05%) into basis points (bps).
/// 1% = 100 bps; 0.01% = 1 bp.
pub fn parse_price_impact_bps(price_impact_pct: &str) -> Result<u16, JupiterError> {
    let pct_f64: f64 = price_impact_pct.parse().map_err(|e| {
        JupiterError::InvalidQuote(format!(
            "Failed to parse price impact pct '{}': {}",
            price_impact_pct, e
        ))
    })?;

    // Negative impact (favorable routing) or 0 is clamped to 0 bps
    if pct_f64 <= 0.0 {
        return Ok(0);
    }

    // 1% is 100 bps, so pct * 100 is bps
    let bps = (pct_f64 * 100.0).round() as u64;
    let clamped_bps = bps.min(10_000) as u16;

    Ok(clamped_bps)
}

/// Computes the effective execution rate: `out_amount / in_amount`.
pub fn calculate_effective_rate(in_amount: u64, out_amount: u64) -> f64 {
    if in_amount == 0 {
        return 0.0;
    }
    (out_amount as f64) / (in_amount as f64)
}

/// Validates that a quote's price impact does not exceed the allowed risk limit.
pub fn validate_quote_price_impact(
    quote: &QuoteResponse,
    max_impact_bps: u16,
) -> Result<BasisPoints, JupiterError> {
    let impact_bps = parse_price_impact_bps(&quote.price_impact_pct)?;

    debug!(
        input_mint = %quote.input_mint,
        output_mint = %quote.output_mint,
        in_amount = %quote.in_amount,
        out_amount = %quote.out_amount,
        impact_bps = impact_bps,
        max_impact_bps = max_impact_bps,
        "Validating quote price impact against policy limit"
    );

    if impact_bps > max_impact_bps {
        return Err(JupiterError::PriceImpactTooHigh {
            actual_bps: impact_bps,
            max_bps: max_impact_bps,
        });
    }

    Ok(BasisPoints(impact_bps))
}
