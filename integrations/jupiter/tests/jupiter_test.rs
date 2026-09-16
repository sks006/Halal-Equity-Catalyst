use equity_catalyst_jupiter::{
    calculate_effective_rate, parse_price_impact_bps, validate_quote_price_impact, JupiterClient,
    JupiterError, QuoteRequest, QuoteResponse, RoutePlanStep, SwapInfo,
};

fn sample_quote() -> QuoteResponse {
    QuoteResponse {
        input_mint: "So11111111111111111111111111111111111111112".to_string(),
        in_amount: "1000000000".to_string(), // 1 SOL
        output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        out_amount: "145000000".to_string(), // 145 USDC
        other_amount_threshold: "144275000".to_string(),
        swap_mode: "ExactIn".to_string(),
        slippage_bps: 50,
        price_impact_pct: "0.05".to_string(),
        route_plan: vec![RoutePlanStep {
            swap_info: SwapInfo {
                amm_key: "Whirlpool111111111111111111111111111111111".to_string(),
                label: Some("Orca Whirlpool".to_string()),
                input_mint: "So11111111111111111111111111111111111111112".to_string(),
                output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                in_amount: "1000000000".to_string(),
                out_amount: "145000000".to_string(),
                fee_amount: Some("500".to_string()),
                fee_mint: Some("So11111111111111111111111111111111111111112".to_string()),
            },
            percent: 100,
        }],
        context_slot: Some(290_000_000),
        time_taken: Some(0.012),
    }
}

#[test]
fn test_price_impact_parsing() {
    assert_eq!(parse_price_impact_bps("0.05").unwrap(), 5);
    assert_eq!(parse_price_impact_bps("1.25").unwrap(), 125);
    assert_eq!(parse_price_impact_bps("0.00").unwrap(), 0);
    assert_eq!(parse_price_impact_bps("-0.02").unwrap(), 0);
    assert_eq!(parse_price_impact_bps("15.50").unwrap(), 1550);
}

#[test]
fn test_price_impact_validation_limits() {
    let quote = sample_quote();

    // Limit is 10 bps, quote impact is 5 bps -> should approve
    let verdict = validate_quote_price_impact(&quote, 10);
    assert!(verdict.is_ok());
    assert_eq!(verdict.unwrap().0, 5);

    // Limit is 2 bps, quote impact is 5 bps -> should reject
    let verdict = validate_quote_price_impact(&quote, 2);
    assert!(verdict.is_err());
    match verdict.unwrap_err() {
        JupiterError::PriceImpactTooHigh {
            actual_bps,
            max_bps,
        } => {
            assert_eq!(actual_bps, 5);
            assert_eq!(max_bps, 2);
        }
        other => panic!("Expected PriceImpactTooHigh error, got {:?}", other),
    }
}

#[test]
fn test_calculate_effective_rate() {
    let in_amount = 1_000_000_000;
    let out_amount = 145_000_000;
    let rate = calculate_effective_rate(in_amount, out_amount);
    assert!((rate - 0.145).abs() < 1e-6);
}

#[tokio::test]
async fn test_mock_client_quote_and_swap() {
    let client = JupiterClient::new_mock();
    let quote = sample_quote();

    client.set_mock_quote(
        "So11111111111111111111111111111111111111112",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        quote.clone(),
    );

    let req = QuoteRequest::new(
        "So11111111111111111111111111111111111111112",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        1_000_000_000,
    );

    let fetched = client
        .get_quote(&req)
        .await
        .expect("Failed to get mock quote");
    assert_eq!(fetched.out_amount, "145000000");

    let swap_req = equity_catalyst_jupiter::build_swap_request(
        &fetched,
        "11111111111111111111111111111111",
        Some(10_000),
    );

    let swap_resp = client
        .build_swap(&swap_req)
        .await
        .expect("Failed to build swap");
    assert!(!swap_resp.swap_transaction.is_empty());
}
