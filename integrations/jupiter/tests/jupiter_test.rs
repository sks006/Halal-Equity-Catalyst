use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use equity_catalyst_jupiter::{
    calculate_effective_rate, decode_swap_transaction, parse_price_impact_bps,
    validate_quote_price_impact, JupiterClient, JupiterError, QuoteRequest, QuoteResponse,
    RoutePlanStep, SwapInfo, SwapResponse,
};
use solana_sdk::message::v0::Message;
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::VersionedTransaction;

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

fn create_valid_serialized_tx(payer: &Pubkey) -> String {
    let msg = Message::try_compile(payer, &[], &[], solana_sdk::hash::Hash::default()).unwrap();
    let tx = VersionedTransaction {
        signatures: vec![solana_sdk::signature::Signature::default()],
        message: VersionedMessage::V0(msg),
    };
    let tx_bytes = bincode::serialize(&tx).unwrap();
    BASE64.encode(tx_bytes)
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
async fn test_mock_client_quote_and_swap_with_real_tx() {
    let client = JupiterClient::new_mock();
    let quote = sample_quote();

    client.set_mock_quote(
        "So11111111111111111111111111111111111111112",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        quote.clone(),
    );

    let authority = Pubkey::new_unique();
    let valid_tx_base64 = create_valid_serialized_tx(&authority);

    client.set_mock_swap(
        "145000000",
        SwapResponse {
            swap_transaction: valid_tx_base64.clone(),
            last_valid_block_height: 250_000_000,
            prioritization_fee_lamports: Some(10_000),
        },
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

    let swap_req =
        equity_catalyst_jupiter::build_swap_request(&fetched, &authority.to_string(), Some(10_000))
            .expect("Should build valid swap request for execution authority");

    let swap_resp = client
        .build_swap(&swap_req)
        .await
        .expect("Failed to build swap");
    assert_eq!(swap_resp.swap_transaction, valid_tx_base64);

    let decoded_tx = decode_swap_transaction(&swap_resp.swap_transaction)
        .expect("Should decode valid VersionedTransaction");
    assert_eq!(decoded_tx.signatures.len(), 1);
}

#[tokio::test]
async fn test_mock_client_swap_without_mock_fails_never_returns_dummy() {
    let client = JupiterClient::new_mock();
    let quote = sample_quote();

    client.set_mock_quote(
        "So11111111111111111111111111111111111111112",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        quote.clone(),
    );

    let authority = Pubkey::new_unique();
    let swap_req =
        equity_catalyst_jupiter::build_swap_request(&quote, &authority.to_string(), Some(5_000))
            .unwrap();

    // With no mock swap configured, client must return error and NEVER return dummy transaction
    let result = client.build_swap(&swap_req).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        JupiterError::InvalidTransaction(msg) => {
            assert!(msg.contains("No mock swap response configured"));
        }
        other => panic!("Expected InvalidTransaction, got {:?}", other),
    }
}

#[test]
fn test_swap_request_validates_execution_authority() {
    let quote = sample_quote();

    // Invalid base58 pubkey
    let invalid = equity_catalyst_jupiter::build_swap_request(&quote, "not_a_valid_pubkey", None);
    assert!(invalid.is_err());

    // Empty authority
    let empty = equity_catalyst_jupiter::build_swap_request(&quote, "   ", None);
    assert!(empty.is_err());

    // Valid execution authority
    let valid_authority = Pubkey::new_unique();
    let valid = equity_catalyst_jupiter::build_swap_request(
        &quote,
        &valid_authority.to_string(),
        Some(1_000),
    );
    assert!(valid.is_ok());
    assert_eq!(valid.unwrap().user_public_key, valid_authority.to_string());
}

#[test]
fn test_decode_swap_transaction_rejects_dummy_payload() {
    let dummy = "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDBg==";
    let res = decode_swap_transaction(dummy);
    assert!(res.is_err());
    assert!(matches!(
        res.unwrap_err(),
        JupiterError::InvalidTransaction(_)
    ));

    let empty = decode_swap_transaction("");
    assert!(empty.is_err());
}
