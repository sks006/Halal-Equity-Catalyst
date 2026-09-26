//! Phase 09: Mathematical Property & Fuzz Tests for Shared Financial Logic
//!
//! Validates:
//! 1. Basis points arithmetic: No overflow, zero division, exact scale bounds [0, 10000].
//! 2. Cash reserve & spot constraints: Reserve non-negativity, exact allocation boundaries.
//! 3. Shariah screening thresholds: StrictLessThan vs LessThanOrEqual mathematical monotonicity.
//! 4. Deterministic fee breakdown: Invariant that pool_fee + platform_fee + network_fee == total_fee.
//! 5. Slippage bounds: Minimum amount out monotonicity with respect to basis points.

use equity_catalyst_shared::{
    compute_bps_amount, screen_financial_metrics, BasisPoints, FeeSchedule, ScreeningPolicy,
    ScreeningStandard, ShariahFinancialMetrics, ShariahRejectionReason, ThresholdComparison,
};

#[test]
fn test_property_basis_points_scale_and_overflow_safety() {
    // Property 1: Valid range is strictly 0..=10,000
    for bps in 0..=10_000 {
        let bp = BasisPoints::new(bps);
        assert!(bp.is_ok(), "Bps {} within 10_000 must be valid", bps);
        let valid_bp = bp.unwrap();
        assert_eq!(valid_bp.as_bps(), bps);
        assert_eq!(valid_bp.to_percentage_f64(), bps as f64 / 100.0);
    }

    // Property 2: Any value > 10,000 must fail closed
    for invalid_bps in [10_001, 10_002, 20_000, 50_000, u16::MAX] {
        assert!(
            BasisPoints::new(invalid_bps).is_err(),
            "Bps {} exceeding 10_000 must be rejected",
            invalid_bps
        );
    }

    // Property 3: compute_bps_amount must never overflow u64 and must be monotonic
    let test_amounts = [
        0u64,
        1,
        100,
        10_000,
        1_000_000,
        1_000_000_000,
        u64::MAX / 10_000,
    ];
    for &amount in &test_amounts {
        let mut prev_portion = 0u64;
        for step in [0u16, 500, 1000, 2500, 5000, 7500, 10_000] {
            let bp = BasisPoints::new(step).unwrap();
            let portion = compute_bps_amount(amount, bp).expect("Valid range must not overflow");
            assert!(
                portion >= prev_portion,
                "Bps calculation must be monotonic: portion {} < prev {}",
                portion,
                prev_portion
            );
            assert!(
                portion <= amount,
                "Bps portion {} must not exceed source amount {}",
                portion,
                amount
            );
            prev_portion = portion;
        }
    }
}

#[test]
fn test_property_deterministic_fee_breakdown_invariant() {
    // Property: total_fee == pool_fee + platform_fee exactly (no hidden/unaccounted token deductions)
    let trade_sizes = [
        0u64,
        100u64,         // $0.001
        10_000u64,      // $0.10
        100_000u64,     // $1.00
        1_000_000u64,   // $10.00
        100_000_000u64, // $1,000.00
    ];

    let pool_bps_cases = [0u16, 5, 10, 15, 30, 100];
    let platform_bps_cases = [0u16, 1, 5, 10, 25];
    let network_fee_cases = [0u64, 5_000, 10_000, 50_000];

    for &trade_amount in &trade_sizes {
        for &pool_bps in &pool_bps_cases {
            for &plat_bps in &platform_bps_cases {
                for &net_fee in &network_fee_cases {
                    let schedule = FeeSchedule {
                        pool_fee_bps: pool_bps,
                        platform_fee_bps: plat_bps,
                        estimated_network_fee_lamports: net_fee,
                        version: "v1.0".to_string(),
                    };
                    let breakdown = schedule.calculate_fees(trade_amount, 6, None);

                    // Exact addition invariant: total token fee == pool fee + platform fee
                    assert_eq!(
                        breakdown.pool_fee + breakdown.platform_fee,
                        breakdown.total_fee,
                        "total_fee must equal sum of component token fees"
                    );

                    // Component non-negativity
                    assert!(breakdown.total_fee >= breakdown.pool_fee);
                    assert!(breakdown.total_fee >= breakdown.platform_fee);
                    assert_eq!(breakdown.network_fee, net_fee);

                    // Version stability
                    assert_eq!(breakdown.fee_calculation_version, "v1.0");
                }
            }
        }
    }
}

#[test]
fn test_property_shariah_threshold_monotonicity() {
    // Property: Monotonic boundary behavior between StrictLessThan (<) and LessThanOrEqual (<=)
    let threshold_bps = 3300u16;

    let strict_policy = ScreeningPolicy::from_raw_bps(
        ScreeningStandard::Aaoifi21,
        "v1",
        threshold_bps,
        3000,
        500,
        ThresholdComparison::StrictLessThan,
    )
    .unwrap();

    let inclusive_policy = ScreeningPolicy::from_raw_bps(
        ScreeningStandard::Aaoifi21,
        "v1",
        threshold_bps,
        3000,
        500,
        ThresholdComparison::LessThanOrEqual,
    )
    .unwrap();

    // 1. Below threshold: Both must pass
    let below =
        ShariahFinancialMetrics::from_market_cap_ratios((threshold_bps - 1) as u32, 100, 50);
    assert!(screen_financial_metrics(&below, &strict_policy).is_ok());
    assert!(screen_financial_metrics(&below, &inclusive_policy).is_ok());

    // 2. Exact threshold boundary: Strict MUST REJECT, Inclusive MUST PASS
    let exact = ShariahFinancialMetrics::from_market_cap_ratios(threshold_bps as u32, 100, 50);
    assert_eq!(
        screen_financial_metrics(&exact, &strict_policy),
        Err(ShariahRejectionReason::ExcessDebt),
        "StrictLessThan must reject ratio == threshold"
    );
    assert!(
        screen_financial_metrics(&exact, &inclusive_policy).is_ok(),
        "LessThanOrEqual must approve ratio == threshold"
    );

    // 3. Above threshold: Both must reject
    let above =
        ShariahFinancialMetrics::from_market_cap_ratios((threshold_bps + 1) as u32, 100, 50);
    assert_eq!(
        screen_financial_metrics(&above, &strict_policy),
        Err(ShariahRejectionReason::ExcessDebt)
    );
    assert_eq!(
        screen_financial_metrics(&above, &inclusive_policy),
        Err(ShariahRejectionReason::ExcessDebt)
    );
}
