use crate::engines::dbc_engine::{config::CurveSegmentConfig, pricing::compute_sqrt_price_q64};

/// Generates piecewise curve segments based on the requested curve profile and initial anchor price.
pub fn generate_curve_segments(
    curve_profile: &str,
    initial_price: f64,
    total_supply: f64,
    base_decimals: u8,
    quote_decimals: u8,
) -> Vec<CurveSegmentConfig> {
    match curve_profile.to_lowercase().as_str() {
        "equity_discovery" | "default" => generate_equity_discovery_segments(
            initial_price,
            total_supply,
            base_decimals,
            quote_decimals,
        ),
        "linear_ramp" => generate_linear_ramp_segments(
            initial_price,
            total_supply,
            base_decimals,
            quote_decimals,
        ),
        _ => generate_equity_discovery_segments(
            initial_price,
            total_supply,
            base_decimals,
            quote_decimals,
        ),
    }
}

/// 3-Regime Equity Discovery Curve:
/// Regime A (1x weight) -> Regime B (4x weight) -> Regime C (8x weight)
fn generate_equity_discovery_segments(
    initial_price: f64,
    total_supply: f64,
    base_decimals: u8,
    quote_decimals: u8,
) -> Vec<CurveSegmentConfig> {
    let p0 = initial_price * 0.85; // 85% floor
    let p1 = initial_price * 1.00; // 100% parity
    let p2 = initial_price * 1.30; // 130% discovery band
    let p3 = initial_price * 1.50; // 150% graduation threshold

    let sqrt_p0 = compute_sqrt_price_q64(p0, base_decimals, quote_decimals);
    let sqrt_p1 = compute_sqrt_price_q64(p1, base_decimals, quote_decimals);
    let sqrt_p2 = compute_sqrt_price_q64(p2, base_decimals, quote_decimals);
    let sqrt_p3 = compute_sqrt_price_q64(p3, base_decimals, quote_decimals);

    let base_liquidity_scale = total_supply * 10_f64.powi(base_decimals as i32) * 1e18;

    vec![
        CurveSegmentConfig {
            segment_index: 0,
            regime_name: "Regime A: Launch / Bootstrapping".to_string(),
            start_price: p0,
            end_price: p1,
            sqrt_price_start: sqrt_p0.to_string(),
            sqrt_price_end: sqrt_p1.to_string(),
            liquidity_weight: 1,
            estimated_liquidity: format!("{:.0}", base_liquidity_scale * 1.0),
            description: "Controlled initial distribution window with moderate slope to mitigate predatory sniping."
                .to_string(),
        },
        CurveSegmentConfig {
            segment_index: 1,
            regime_name: "Regime B: Active Price Discovery".to_string(),
            start_price: p1,
            end_price: p2,
            sqrt_price_start: sqrt_p1.to_string(),
            sqrt_price_end: sqrt_p2.to_string(),
            liquidity_weight: 4,
            estimated_liquidity: format!("{:.0}", base_liquidity_scale * 4.0),
            description: "4x concentrated liquidity centered around consensus fair value. Low slippage for institutional block discovery."
                .to_string(),
        },
        CurveSegmentConfig {
            segment_index: 2,
            regime_name: "Regime C: Mature Market Buffer".to_string(),
            start_price: p2,
            end_price: p3,
            sqrt_price_start: sqrt_p2.to_string(),
            sqrt_price_end: sqrt_p3.to_string(),
            liquidity_weight: 8,
            estimated_liquidity: format!("{:.0}", base_liquidity_scale * 8.0),
            description: "8x concentrated liquidity pre-graduation stabilization. Eliminates terminal volatility before DAMM v2 migration."
                .to_string(),
        },
    ]
}

/// Fallback linear ramp with equal weights across segments.
fn generate_linear_ramp_segments(
    initial_price: f64,
    total_supply: f64,
    base_decimals: u8,
    quote_decimals: u8,
) -> Vec<CurveSegmentConfig> {
    let p0 = initial_price * 0.90;
    let p1 = initial_price * 1.10;
    let p2 = initial_price * 1.30;

    let sqrt_p0 = compute_sqrt_price_q64(p0, base_decimals, quote_decimals);
    let sqrt_p1 = compute_sqrt_price_q64(p1, base_decimals, quote_decimals);
    let sqrt_p2 = compute_sqrt_price_q64(p2, base_decimals, quote_decimals);

    let base_liquidity_scale = total_supply * 10_f64.powi(base_decimals as i32) * 1e18;

    vec![
        CurveSegmentConfig {
            segment_index: 0,
            regime_name: "Segment 1: Lower Band".to_string(),
            start_price: p0,
            end_price: p1,
            sqrt_price_start: sqrt_p0.to_string(),
            sqrt_price_end: sqrt_p1.to_string(),
            liquidity_weight: 1,
            estimated_liquidity: format!("{:.0}", base_liquidity_scale),
            description: "Linear ramp entry band.".to_string(),
        },
        CurveSegmentConfig {
            segment_index: 1,
            regime_name: "Segment 2: Upper Band".to_string(),
            start_price: p1,
            end_price: p2,
            sqrt_price_start: sqrt_p1.to_string(),
            sqrt_price_end: sqrt_p2.to_string(),
            liquidity_weight: 1,
            estimated_liquidity: format!("{:.0}", base_liquidity_scale),
            description: "Linear ramp graduation band.".to_string(),
        },
    ]
}
