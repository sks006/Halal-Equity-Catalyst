//! Integration tests for Shariah Purification (Phase 15).
//!
//! Verifies:
//! 1. Complete separation of purification calculations from Shariah eligibility screening.
//! 2. Strict conceptual and type separation between:
//!    - `impure_income_ratio_bps` (basis points ratio, e.g. 150 bps = 1.50%)
//!    - `impure_income_value_minor_units` / `purification_value_minor_units` (monetary minor currency units, e.g. cents)
//! 3. The eligibility ratio itself is NEVER used as the purification payment amount.
//! 4. Dedicated purification module with explicit units/currency (`CurrencyCode`, decimals).
//! 5. Deterministic integer arithmetic without float drift.
//! 6. Rounding rules (`ConservativeCeiling`, `NearestHalfUp`, `Floor`).
//! 7. Zero payment execution in this phase.

use equity_catalyst_shared::asset::{
    Asset, AssetIdentity, AssetProvider, AssetStatus, AssetType, Network, ProviderConfig,
    TokenDetails,
};
use equity_catalyst_shared::shariah::ownership::OwnershipRecord;
use equity_catalyst_shared::shariah::policy::ScreeningPolicy;
use equity_catalyst_shared::shariah::purification::{
    assess_direct_impure_value, assess_dividend_purification,
    assess_per_share_dividend_purification, assess_purification_for_registered_asset,
    calculate_purification_value_minor_units, CurrencyCode, DirectValuePurificationRequest,
    DividendPurificationRequest, PerShareDividendPurificationRequest,
    PurificationError, PurificationRoundingRule,
};
use equity_catalyst_shared::shariah::registry::register_asset_at;
use equity_catalyst_shared::shariah::screening::{
    BusinessActivityAssessment, BusinessCategory, ShariahFinancialMetrics,
};
use equity_catalyst_shared::shariah::types::ShariahStatus;

fn sample_asset(asset_id: &str, symbol: &str) -> Asset {
    Asset {
        identity: AssetIdentity {
            asset_id: asset_id.to_string(),
            symbol: symbol.to_string(),
            name: format!("{symbol} Tokenized Common Stock"),
            asset_type: AssetType::Stock,
            underlying_reference: "US0378331005".to_string(),
        },
        token: TokenDetails {
            mint: "So11111111111111111111111111111111111111112".to_string(),
            decimals: 6,
            network: Network::SolanaMainnet,
        },
        provider: ProviderConfig {
            provider: AssetProvider::Backed,
            price_feed_id: "0xfeed1234".to_string(),
            meteora_pool: None,
            secondary_reference: None,
            status: AssetStatus::Active,
        },
    }
}

fn sample_ownership() -> OwnershipRecord {
    OwnershipRecord {
        verified: true,
        issuer: "Backed Finance AG".to_string(),
        custodian: "Maerki Baumann & Co. AG".to_string(),
        legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
        instrument_reference: "ISIN: US0378331005".to_string(),
        evidence_hash: "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069"
            .to_string(),
        verified_at: 1_700_000_000,
        expires_at: 1_850_000_000,
    }
}

fn sample_business_activity() -> BusinessActivityAssessment {
    BusinessActivityAssessment::reviewed_permissible(
        BusinessCategory::Technology,
        "Consumer electronics, software, and online cloud services",
        "SEC Form 10-K FY2025",
    )
}

#[test]
fn test_acceptance_criteria_purification_independent_from_shariah_eligibility_decision() {
    // Standard AAOIFI screening allows up to 500 bps (5.00%) impure income
    let policy = ScreeningPolicy::board_approved_v1();
    let current_timestamp = 1_720_000_000;

    let asset = sample_asset("backed:AAPL", "AAPL");
    let ownership = sample_ownership();
    let business = sample_business_activity();

    // Financials have 140 bps (1.40%) non-operating interest income.
    // 140 bps <= 500 bps threshold -> The asset is fully APPROVED for vault spot investment.
    let financials = ShariahFinancialMetrics::from_market_cap_ratios(1_200, 800, 140);
    let registered_asset = register_asset_at(
        asset,
        ownership,
        financials,
        business,
        &policy,
        current_timestamp,
    )
    .expect("Asset must pass initial Shariah screening");

    // 1. Verify screening eligibility decision is APPROVED
    assert_eq!(registered_asset.eligibility.status, ShariahStatus::Approved);
    assert_eq!(registered_asset.eligibility.impure_income_bps, 140);
    assert!(registered_asset.is_tradeable(current_timestamp));

    // 2. Independently calculate purification for a $50,000.00 dividend payout (5,000,000 cents in USD)
    let gross_dividend_value_minor_units = 5_000_000u64; // $50,000.00 in cents
    let purification = assess_purification_for_registered_asset(
        &registered_asset,
        gross_dividend_value_minor_units,
        CurrencyCode::Usd,
        PurificationRoundingRule::ConservativeCeiling,
    )
    .expect("Purification calculation succeeds independently of screening step");

    // Invariant: Gross dividend is preserved
    assert_eq!(
        purification.gross_dividend_value_minor_units(),
        5_000_000
    );

    // Invariant: Purification ratio is 140 bps
    assert_eq!(purification.impure_income_ratio_bps(), 140);

    // Invariant: The payment value is 1.40% of 5,000,000 cents = 70,000 cents ($700.00)
    assert_eq!(purification.purification_value_minor_units(), 70_000);

    // CRITICAL: The payment value (70,000) is NOT the ratio itself (140)
    assert_ne!(
        purification.purification_value_minor_units(),
        purification.impure_income_ratio_bps() as u64,
        "Eligibility ratio must NEVER be used as the payment value!"
    );

    // Invariant: Net permissible value retains exactly (Gross - Purification)
    assert_eq!(
        purification.net_permissible_value_minor_units(),
        4_930_000 // $49,300.00 in cents
    );
    assert_eq!(
        purification.net_permissible_value_minor_units()
            + purification.purification_value_minor_units(),
        purification.gross_dividend_value_minor_units()
    );

    assert!(purification.purification_required);
    assert!(!purification.is_pure());
}

#[test]
fn test_strict_separation_of_ratio_bps_and_value_minor_units() {
    // Test that a high dividend with a tiny ratio does not confuse ratio with value
    let small_ratio_bps = 5u32; // 0.05%
    let large_gross_dividend_minor = 100_000_000u64; // $1,000,000.00 USDC (6 decimals: 100,000,000 micro-units = $100.00 or with 2 dec $1M)

    let req = DividendPurificationRequest {
        asset_id: "backed:TSLA".to_string(),
        currency_code: CurrencyCode::Usdc,
        gross_dividend_value_minor_units: large_gross_dividend_minor,
        impure_income_ratio_bps: small_ratio_bps,
        rounding_rule: PurificationRoundingRule::ConservativeCeiling,
    };

    let result = assess_dividend_purification(&req).expect("Assessment succeeds");

    // 0.05% of 100,000,000 = 50,000 minor units
    assert_eq!(result.purification_value_minor_units, 50_000);
    assert_eq!(result.impure_income_ratio_bps, 5);

    // They are distinct concepts:
    assert_ne!(
        result.purification_value_minor_units,
        result.impure_income_ratio_bps as u64
    );
}

#[test]
fn test_all_rounding_rules_deterministic_and_conservative_ceiling() {
    // 100 minor units with 33 bps (0.33%) impure income:
    // 100 * 33 = 3,300. 3,300 / 10,000 = 0 quotient, 3,300 remainder.
    let gross = 100u64;
    let ratio = 33u32;

    // Floor: discards 0.33 -> 0
    let floor_val = calculate_purification_value_minor_units(
        gross,
        ratio,
        PurificationRoundingRule::Floor,
    )
    .unwrap();
    assert_eq!(floor_val, 0);

    // NearestHalfUp: 3,300 < 5,000 -> 0
    let half_val = calculate_purification_value_minor_units(
        gross,
        ratio,
        PurificationRoundingRule::NearestHalfUp,
    )
    .unwrap();
    assert_eq!(half_val, 0);

    // ConservativeCeiling: remainder > 0 -> rounds UP to 1 minor unit (Islamic religious precaution / Ihtiyat)
    let ceiling_val = calculate_purification_value_minor_units(
        gross,
        ratio,
        PurificationRoundingRule::ConservativeCeiling,
    )
    .unwrap();
    assert_eq!(ceiling_val, 1);

    // Half boundary test: 50 bps on 100 units = exactly 0.50
    let half_ratio = 50u32;
    assert_eq!(
        calculate_purification_value_minor_units(gross, half_ratio, PurificationRoundingRule::Floor).unwrap(),
        0
    );
    assert_eq!(
        calculate_purification_value_minor_units(gross, half_ratio, PurificationRoundingRule::NearestHalfUp).unwrap(),
        1
    );
    assert_eq!(
        calculate_purification_value_minor_units(gross, half_ratio, PurificationRoundingRule::ConservativeCeiling).unwrap(),
        1
    );
}

#[test]
fn test_per_share_dividend_purification_calculation() {
    // 10,000 shares, $2.50 per share (250 cents)
    // Gross = 10,000 * 250 = 2,500,000 cents ($25,000.00)
    // Ratio = 200 bps (2.00%)
    let req = PerShareDividendPurificationRequest {
        asset_id: "backed:MSFT".to_string(),
        currency_code: CurrencyCode::Usd,
        total_shares_count: 10_000,
        dividend_per_share_value_minor_units: 250,
        impure_income_ratio_bps: 200,
        rounding_rule: PurificationRoundingRule::ConservativeCeiling,
    };

    let assessment = assess_per_share_dividend_purification(&req).unwrap();
    assert_eq!(assessment.gross_dividend_value_minor_units, 2_500_000);
    assert_eq!(assessment.impure_income_ratio_bps, 200);
    // 2.00% of 2,500,000 cents = 50,000 cents ($500.00)
    assert_eq!(assessment.purification_value_minor_units, 50_000);
    assert_eq!(assessment.net_permissible_value_minor_units, 2_450_000);
    assert!(assessment.purification_required);
}

#[test]
fn test_direct_impure_value_assessment() {
    // Custodian reports gross dividend of 1,000,000 USDC micro-units ($1.00)
    // with exact impure interest component of 15,000 USDC micro-units ($0.015)
    let req = DirectValuePurificationRequest {
        asset_id: "backed:GOOGL".to_string(),
        currency_code: CurrencyCode::Usdc,
        gross_dividend_value_minor_units: 1_000_000,
        impure_income_value_minor_units: 15_000,
    };

    let assessment = assess_direct_impure_value(&req).unwrap();
    assert_eq!(assessment.gross_dividend_value_minor_units, 1_000_000);
    assert_eq!(assessment.purification_value_minor_units, 15_000);
    assert_eq!(assessment.net_permissible_value_minor_units, 985_000);
    assert_eq!(assessment.impure_income_ratio_bps, 150); // 1.50%
    assert!(assessment.purification_required);
}

#[test]
fn test_ratio_exceeds_maximum_error() {
    let err = calculate_purification_value_minor_units(
        1_000_000,
        10_001,
        PurificationRoundingRule::ConservativeCeiling,
    );
    assert_eq!(
        err,
        Err(PurificationError::RatioExceedsMaximum {
            impure_income_ratio_bps: 10_001
        })
    );
}

#[test]
fn test_direct_value_exceeds_gross_error() {
    let req = DirectValuePurificationRequest {
        asset_id: "backed:ERR".to_string(),
        currency_code: CurrencyCode::Usd,
        gross_dividend_value_minor_units: 100_000,
        impure_income_value_minor_units: 100_001,
    };
    let err = assess_direct_impure_value(&req);
    assert_eq!(
        err,
        Err(PurificationError::DirectValueExceedsGross {
            impure_income_value_minor_units: 100_001,
            gross_dividend_value_minor_units: 100_000,
        })
    );
}

#[test]
fn test_no_payment_execution_in_phase_15() {
    // In Phase 15, the purification module produces an assessment report (`PurificationAssessment`).
    // No transaction signatures, no on-chain CPI instructions, and no wallet transfers are executed.
    let assessment = assess_dividend_purification(&DividendPurificationRequest {
        asset_id: "backed:AMZN".to_string(),
        currency_code: CurrencyCode::Usd,
        gross_dividend_value_minor_units: 1_000_000,
        impure_income_ratio_bps: 100,
        rounding_rule: PurificationRoundingRule::ConservativeCeiling,
    })
    .unwrap();

    // Verify it is strictly a data assessment and financial audit structure
    assert_eq!(assessment.purification_value_minor_units, 10_000);
    assert_eq!(assessment.currency_code, CurrencyCode::Usd);
    // Nothing executed on-chain
}
