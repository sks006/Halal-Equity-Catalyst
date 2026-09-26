//! Phase 9 Tests: DEX Quoter abstraction and executable market quotes.
//!
//! Validates:
//! 1. Valid executable quote acquisition with all required identity, pricing, and routing fields.
//! 2. Expiration and staleness rejection logic.
//! 3. Zero/invalid trade amount rejection.
//! 4. Excessive price impact rejection against risk limits.
//! 5. Unsupported asset pair rejection.
//! 6. Native `JupiterClient` implementation of `DexQuoter`.
//! 7. Full acceptance criteria: Independent executable DEX quote acquisition separate from Pyth oracle price.

use chrono::Utc;
use equity_catalyst_jupiter::{
    DexQuoteError, DexQuoteRequest, DexQuoter, JupiterClient, MockDexQuoter, QuoteResponse,
    RoutePlanStep, SwapInfo,
};

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const BACKED_AAPL_MINT: &str = "AAPL111111111111111111111111111111111111111";

#[tokio::test]
async fn test_dex_quoter_valid_quote() {
    let quoter = MockDexQuoter::new("mock-dex");
    // 1 SOL = 145 USDC (0.145 USDC per lamport where USDC has 6 decimals and SOL has 9 decimals)
    quoter.add_pair(SOL_MINT, USDC_MINT, 0.145, 8); // 8 bps impact

    let now = Utc::now().timestamp();
    quoter.set_fixed_timestamp(Some(now));

    let req = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000) // 1 SOL
        .with_slippage_bps(50)
        .with_max_price_impact_bps(100)
        .with_ttl_seconds(20);

    let quote = quoter
        .get_executable_quote(&req)
        .await
        .expect("Valid quote should succeed");

    // Verify required fields (Task 3)
    assert_eq!(quote.input_mint, SOL_MINT);
    assert_eq!(quote.output_mint, USDC_MINT);
    assert_eq!(quote.input_amount, 1_000_000_000);
    assert_eq!(quote.expected_output_amount, 145_000_000); // 145 USDC
                                                           // Minimum output with 50 bps (0.5%) slippage: 145_000_000 * 0.995 = 144_275_000
    assert_eq!(quote.minimum_output_amount, 144_275_000);
    assert_eq!(quote.price_impact_bps, 8);
    assert_eq!(quote.price_impact_pct, "0.08%");
    assert_eq!(quote.provider_id, "mock-dex");
    assert_eq!(quote.quote_timestamp, now);
    assert_eq!(quote.expires_at, now + 20);
    assert_eq!(quote.ttl_seconds, 20);

    // Verify route information
    assert_eq!(quote.route_info.num_hops, 1);
    assert_eq!(quote.route_info.primary_dex, "mock-dex");
    assert_eq!(quote.route_info.steps.len(), 1);
    assert_eq!(quote.route_info.steps[0].in_amount, 1_000_000_000);
    assert_eq!(quote.route_info.steps[0].out_amount, 145_000_000);

    // Expiry check
    assert!(!quote.is_expired(now));
    assert!(quote.validate_freshness(now).is_ok());
}

#[tokio::test]
async fn test_dex_quoter_expired_quote() {
    let quoter = MockDexQuoter::new("mock-dex");
    quoter.add_pair(SOL_MINT, USDC_MINT, 0.145, 5);

    let base_time = 1700000000;
    quoter.set_fixed_timestamp(Some(base_time));

    let req = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000).with_ttl_seconds(10);
    let quote = quoter.get_executable_quote(&req).await.unwrap();

    assert_eq!(quote.quote_timestamp, base_time);
    assert_eq!(quote.expires_at, base_time + 10);

    // Fresh at base_time + 5
    assert!(!quote.is_expired(base_time + 5));
    assert!(quote.validate_freshness(base_time + 5).is_ok());

    // Expired at base_time + 10 (exact boundary)
    assert!(quote.is_expired(base_time + 10));
    let err_exact = quote.validate_freshness(base_time + 10);
    assert!(matches!(
        err_exact,
        Err(DexQuoteError::ExpiredQuote {
            quote_timestamp: 1700000000,
            expires_at: 1700000010,
            current_time: 1700000010,
        })
    ));

    // Expired at base_time + 15
    assert!(quote.is_expired(base_time + 15));
    let err_stale = quote.validate_freshness(base_time + 15);
    assert!(matches!(err_stale, Err(DexQuoteError::ExpiredQuote { .. })));

    // Forced expired quoter simulation
    quoter.set_forced_expired(true);
    let forced_stale = quoter.get_executable_quote(&req).await.unwrap();
    assert!(forced_stale.is_expired(base_time));
    assert!(forced_stale.validate_freshness(base_time).is_err());
}

#[tokio::test]
async fn test_dex_quoter_invalid_amount() {
    let quoter = MockDexQuoter::new("mock-dex");
    quoter.add_pair(SOL_MINT, USDC_MINT, 0.145, 5);

    // Amount = 0 must be rejected
    let req = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 0);
    let result = quoter.get_executable_quote(&req).await;

    assert!(matches!(result, Err(DexQuoteError::InvalidAmount(_))));
}

#[tokio::test]
async fn test_dex_quoter_excessive_price_impact() {
    let quoter = MockDexQuoter::new("mock-dex");
    // High impact pair: 250 bps (2.50%)
    quoter.add_pair(SOL_MINT, BACKED_AAPL_MINT, 0.65, 250);

    // Request with max impact 100 bps (1.00%) -> must reject
    let req_strict = DexQuoteRequest::new(SOL_MINT, BACKED_AAPL_MINT, 10_000_000_000)
        .with_max_price_impact_bps(100);

    let result_strict = quoter.get_executable_quote(&req_strict).await;
    assert_eq!(
        result_strict,
        Err(DexQuoteError::ExcessivePriceImpact {
            actual_bps: 250,
            max_bps: 100,
        })
    );

    // Request with max impact 300 bps (3.00%) -> must succeed
    let req_lenient = DexQuoteRequest::new(SOL_MINT, BACKED_AAPL_MINT, 10_000_000_000)
        .with_max_price_impact_bps(300);

    let result_lenient = quoter.get_executable_quote(&req_lenient).await;
    assert!(result_lenient.is_ok());
    assert_eq!(result_lenient.unwrap().price_impact_bps, 250);
}

#[tokio::test]
async fn test_dex_quoter_unsupported_pair() {
    let quoter = MockDexQuoter::new("mock-dex");
    // Only SOL -> USDC is supported
    quoter.add_pair(SOL_MINT, USDC_MINT, 0.145, 5);

    let unsupported_input = "UnknownTokenMint11111111111111111111111111";
    let req = DexQuoteRequest::new(unsupported_input, USDC_MINT, 1_000_000_000);

    let result = quoter.get_executable_quote(&req).await;
    assert_eq!(
        result,
        Err(DexQuoteError::UnsupportedPair {
            input_mint: unsupported_input.to_string(),
            output_mint: USDC_MINT.to_string(),
        })
    );
}

#[tokio::test]
async fn test_jupiter_client_dex_quoter_implementation() {
    let client = JupiterClient::new_mock();

    // Mock Jupiter quote response
    let jup_quote = QuoteResponse {
        input_mint: SOL_MINT.to_string(),
        in_amount: "1000000000".to_string(),
        output_mint: USDC_MINT.to_string(),
        out_amount: "148500000".to_string(),
        other_amount_threshold: "147757500".to_string(),
        swap_mode: "ExactIn".to_string(),
        slippage_bps: 50,
        price_impact_pct: "0.04".to_string(),
        route_plan: vec![RoutePlanStep {
            swap_info: SwapInfo {
                amm_key: "OrcaWhirlpoolKey1111111111111111111111111".to_string(),
                label: Some("Orca Whirlpool".to_string()),
                input_mint: SOL_MINT.to_string(),
                output_mint: USDC_MINT.to_string(),
                in_amount: "1000000000".to_string(),
                out_amount: "148500000".to_string(),
                fee_amount: Some("500".to_string()),
                fee_mint: Some(SOL_MINT.to_string()),
            },
            percent: 100,
        }],
        context_slot: Some(290_000_000),
        time_taken: Some(0.015),
    };

    client.set_mock_quote(SOL_MINT, USDC_MINT, jup_quote);

    // Call through the DexQuoter trait interface
    let quoter: &dyn DexQuoter = &client;
    assert_eq!(quoter.provider_id(), "jupiter");

    let req = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000)
        .with_slippage_bps(50)
        .with_max_price_impact_bps(100)
        .with_ttl_seconds(15);

    let dex_quote = quoter
        .get_executable_quote(&req)
        .await
        .expect("Jupiter DexQuoter should return executable quote");

    assert_eq!(dex_quote.provider_id, "jupiter");
    assert_eq!(dex_quote.input_mint, SOL_MINT);
    assert_eq!(dex_quote.output_mint, USDC_MINT);
    assert_eq!(dex_quote.input_amount, 1_000_000_000);
    assert_eq!(dex_quote.expected_output_amount, 148_500_000);
    assert_eq!(dex_quote.minimum_output_amount, 147_757_500);
    assert_eq!(dex_quote.price_impact_bps, 4);
    assert_eq!(dex_quote.route_info.primary_dex, "Orca Whirlpool");
    assert_eq!(dex_quote.route_info.steps.len(), 1);
    assert_eq!(dex_quote.ttl_seconds, 15);
    assert!(!dex_quote.is_expired(Utc::now().timestamp()));
}

#[tokio::test]
async fn test_phase9_acceptance_criteria_independent_dex_quote_vs_pyth_oracle() {
    // ACCEPTANCE CRITERIA:
    // The system can obtain an executable DEX quote independently of Pyth.
    //
    // Core Principle:
    // Pyth = reference / oracle price (macro benchmark, no liquidity/AMM routing)
    // DEX quote = executable price (atomic units, route plan, minimum guaranteed output)
    // Never substitute one for the other.

    // 1. Pyth reference price representation (e.g. from MarketDataStore / Phase 6)
    let pyth_reference_price_scaled: u64 = 150_000_000; // $150.000000 reference USD
    let pyth_reference_price_usd: f64 = 150.0;

    // 2. DEX quoter setup (operates independently, without accessing Pyth)
    let dex_quoter = MockDexQuoter::new("orca-whirlpool");
    // Actual DEX AMM pool liquidity converts 1 SOL ($150 reference) with real slippage to $149.80
    dex_quoter.add_pair(SOL_MINT, USDC_MINT, 0.1498, 12); // 12 bps price impact

    let now = Utc::now().timestamp();
    dex_quoter.set_fixed_timestamp(Some(now));

    let trade_request = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000) // 1 SOL
        .with_slippage_bps(50) // 0.50% slippage
        .with_max_price_impact_bps(50) // max 0.50% impact
        .with_ttl_seconds(15);

    // 3. Obtain executable DEX quote INDEPENDENTLY of Pyth
    let executable_quote = dex_quoter
        .get_executable_quote(&trade_request)
        .await
        .expect("DEX quote should be obtained independently of Pyth");

    // 4. Verify separation of concerns
    // A. Reference price provides baseline
    assert_eq!(pyth_reference_price_usd, 150.0);
    assert_eq!(pyth_reference_price_scaled, 150_000_000);

    // B. Executable quote contains execution mechanics (NOT provided by Pyth)
    assert_eq!(executable_quote.expected_output_amount, 149_800_000); // $149.80 executable
    assert_eq!(executable_quote.minimum_output_amount, 149_051_000); // $149.051 minimum
    assert_eq!(executable_quote.price_impact_bps, 12); // 0.12% impact
    assert_eq!(executable_quote.route_info.primary_dex, "orca-whirlpool");
    assert_eq!(executable_quote.expires_at, now + 15);

    // C. Confirm executable price is different from reference price due to liquidity dynamics
    let executable_usd = executable_quote.expected_output_amount as f64 / 1_000_000.0;
    assert_ne!(executable_usd, pyth_reference_price_usd);
    assert!(executable_usd < pyth_reference_price_usd);

    // D. Valid freshness check enforces execution window
    assert!(executable_quote.validate_freshness(now).is_ok());
    assert!(executable_quote.validate_freshness(now + 20).is_err());
}

#[tokio::test]
async fn test_phase_p5_quote_contains_all_9_fields_and_accessors() {
    let client = JupiterClient::new_mock();

    let jup_quote = QuoteResponse {
        input_mint: SOL_MINT.to_string(),
        in_amount: "2000000000".to_string(),
        output_mint: USDC_MINT.to_string(),
        out_amount: "290000000".to_string(),
        other_amount_threshold: "288550000".to_string(),
        swap_mode: "ExactIn".to_string(),
        slippage_bps: 50,
        price_impact_pct: "0.03".to_string(),
        route_plan: vec![RoutePlanStep {
            swap_info: SwapInfo {
                amm_key: "WhirlpoolPool111111111111111111111111111111".to_string(),
                label: Some("Orca".to_string()),
                input_mint: SOL_MINT.to_string(),
                output_mint: USDC_MINT.to_string(),
                in_amount: "2000000000".to_string(),
                out_amount: "290000000".to_string(),
                fee_amount: Some("1000".to_string()),
                fee_mint: Some(SOL_MINT.to_string()),
            },
            percent: 100,
        }],
        context_slot: Some(300_000_000),
        time_taken: Some(0.010),
    };

    client.set_mock_quote(SOL_MINT, USDC_MINT, jup_quote);

    let req = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 2_000_000_000)
        .with_slippage_bps(50)
        .with_max_price_impact_bps(50)
        .with_ttl_seconds(30);

    let quote = client.get_executable_quote(&req).await.unwrap();

    // Verify all 9 required fields via accessors (Task 3)
    assert_eq!(quote.input_mint(), SOL_MINT);
    assert_eq!(quote.output_mint(), USDC_MINT);
    assert_eq!(quote.input_amount(), 2_000_000_000);
    assert_eq!(quote.expected_output(), 290_000_000);
    assert_eq!(quote.minimum_output(), 288_550_000);
    assert_eq!(quote.route().steps.len(), 1);
    assert_eq!(quote.route().primary_dex, "Orca");
    assert_eq!(quote.price_impact(), 3); // 3 bps
    assert!(quote.quote_timestamp() <= Utc::now().timestamp());
    assert_eq!(quote.expiration(), quote.quote_timestamp() + 30);

    // Validate all invariants pass
    assert!(quote.validate(Some(50), quote.quote_timestamp()).is_ok());
}

#[tokio::test]
async fn test_phase_p5_quote_rejects_missing_route() {
    let client = JupiterClient::new_mock();

    let no_route_quote = QuoteResponse {
        input_mint: SOL_MINT.to_string(),
        in_amount: "1000000000".to_string(),
        output_mint: USDC_MINT.to_string(),
        out_amount: "145000000".to_string(),
        other_amount_threshold: "144000000".to_string(),
        swap_mode: "ExactIn".to_string(),
        slippage_bps: 50,
        price_impact_pct: "0.01".to_string(),
        route_plan: vec![], // Empty route plan
        context_slot: None,
        time_taken: None,
    };

    client.set_mock_quote(SOL_MINT, USDC_MINT, no_route_quote);

    let req = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000);
    let result = client.get_executable_quote(&req).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        DexQuoteError::NoRoute {
            input_mint,
            output_mint,
        } => {
            assert_eq!(input_mint, SOL_MINT);
            assert_eq!(output_mint, USDC_MINT);
        }
        other => panic!("Expected NoRoute error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_phase_p5_quote_rejects_zero_output_amounts() {
    let client = JupiterClient::new_mock();

    let zero_out_quote = QuoteResponse {
        input_mint: SOL_MINT.to_string(),
        in_amount: "1000000000".to_string(),
        output_mint: USDC_MINT.to_string(),
        out_amount: "0".to_string(), // Zero out amount
        other_amount_threshold: "0".to_string(),
        swap_mode: "ExactIn".to_string(),
        slippage_bps: 50,
        price_impact_pct: "0.01".to_string(),
        route_plan: vec![RoutePlanStep {
            swap_info: SwapInfo {
                amm_key: "WhirlpoolPool111111111111111111111111111111".to_string(),
                label: Some("Orca".to_string()),
                input_mint: SOL_MINT.to_string(),
                output_mint: USDC_MINT.to_string(),
                in_amount: "1000000000".to_string(),
                out_amount: "0".to_string(),
                fee_amount: None,
                fee_mint: None,
            },
            percent: 100,
        }],
        context_slot: None,
        time_taken: None,
    };

    client.set_mock_quote(SOL_MINT, USDC_MINT, zero_out_quote);

    let req = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000);
    let result = client.get_executable_quote(&req).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        DexQuoteError::InvalidAmount(_)
    ));
}

#[tokio::test]
async fn test_phase_p5_quote_validates_mints() {
    let client = JupiterClient::new_mock();

    // 1. Identical input and output mint
    let req_same = DexQuoteRequest::new(SOL_MINT, SOL_MINT, 1_000_000_000);
    let res_same = client.get_executable_quote(&req_same).await;
    assert!(matches!(
        res_same,
        Err(DexQuoteError::UnsupportedPair { .. })
    ));

    // 2. Invalid base58 pubkey
    let req_invalid = DexQuoteRequest::new("invalid_pubkey", USDC_MINT, 1_000_000_000);
    let res_invalid = client.get_executable_quote(&req_invalid).await;
    assert!(matches!(res_invalid, Err(DexQuoteError::InvalidMint(_))));

    // 3. Supported mints whitelist filtering
    client.set_supported_mints(vec![SOL_MINT.to_string()]); // USDC not in whitelist
    let req_unsupported = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000);
    let res_unsupported = client.get_executable_quote(&req_unsupported).await;
    assert!(matches!(
        res_unsupported,
        Err(DexQuoteError::UnsupportedPair { .. })
    ));

    // Clear supported mints whitelist -> allows any valid pubkey
    client.clear_supported_mints();
    assert!(client.is_mint_supported(USDC_MINT));
}
