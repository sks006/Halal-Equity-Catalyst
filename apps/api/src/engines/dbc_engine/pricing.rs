/// Fixed-point Q64.64 precision constant (2^64).
pub const Q64: f64 = 18446744073709551616.0; // 2^64

/// Computes the square root price in Q64.64 fixed-point format as expected by Meteora DBC on-chain.
///
/// Formula: sqrt_price_q64 = round(sqrt(price * 10^(quote_decimals - base_decimals)) * 2^64)
pub fn compute_sqrt_price_q64(price: f64, base_decimals: u8, quote_decimals: u8) -> u128 {
    let decimal_adjustment = 10_f64.powi(quote_decimals as i32 - base_decimals as i32);
    let adjusted_price = price * decimal_adjustment;
    let sqrt_price = adjusted_price.sqrt();
    let q64_val = sqrt_price * Q64;
    q64_val.round() as u128
}

/// Converts a Q64.64 square root price back to regular price float.
pub fn price_from_sqrt_price_q64(
    sqrt_price_q64: u128,
    base_decimals: u8,
    quote_decimals: u8,
) -> f64 {
    let sqrt_price = (sqrt_price_q64 as f64) / Q64;
    let raw_price = sqrt_price * sqrt_price;
    let decimal_adjustment = 10_f64.powi(quote_decimals as i32 - base_decimals as i32);
    raw_price / decimal_adjustment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sqrt_price_q64_identity() {
        let price = 100.0;
        let q64 = compute_sqrt_price_q64(price, 6, 6);
        let recovered = price_from_sqrt_price_q64(q64, 6, 6);
        assert!((recovered - price).abs() < 1e-4);
    }
}
