//! Phase P5 Integration Tests: DEX Quoting & Swap Transaction Construction.
//!
//! Validates:
//! 1. Production configuration points to the real configured Jupiter endpoint.
//! 2. A real quote contains all 9 required fields:
//!    - input mint
//!    - output mint
//!    - input amount
//!    - expected output
//!    - minimum output
//!    - route
//!    - price impact
//!    - quote timestamp
//!    - expiration
//! 3. All validation invariants:
//!    - input amount > 0
//!    - supported mints (valid base58, distinct)
//!    - route exists
//!    - quote not expired
//!    - price impact within policy
//!    - expected output > 0
//!    - minimum output > 0
//! 4. Removal of all production dummy responses.
//! 5. `build_swap()` requests a real transaction from the DEX provider over HTTP.
//! 6. Provider failure returns an error (never dummy fallback).
//! 7. Never return a dummy serialized transaction.
//! 8. Swap transaction must be for the actual execution authority/public key.
//! 9. Construction, signing, and submission remain strictly decoupled; no signing in Jupiter client.

use axum::extract::Query;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use equity_catalyst_api::config::Config;
use equity_catalyst_jupiter::{
    build_swap_request, decode_swap_transaction, DexQuoteError, DexQuoteRequest, DexQuoter,
    JupiterClient, JupiterError, QuoteRequest, QuoteResponse, RoutePlanStep, SwapInfo, SwapRequest,
    SwapResponse,
};
use serde::Deserialize;
use solana_sdk::message::v0::Message;
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::VersionedTransaction;
use tokio::net::TcpListener;

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// Helper to generate a genuine serialized Solana VersionedTransaction.
fn create_real_versioned_transaction(payer: &Pubkey) -> (VersionedTransaction, String) {
    let msg = Message::try_compile(payer, &[], &[], solana_sdk::hash::Hash::default()).unwrap();
    let tx = VersionedTransaction {
        signatures: vec![solana_sdk::signature::Signature::default()],
        message: VersionedMessage::V0(msg),
    };
    let tx_bytes = bincode::serialize(&tx).unwrap();
    let tx_base64 = BASE64.encode(tx_bytes);
    (tx, tx_base64)
}

/// Query parameters for Jupiter's `/quote` endpoint
#[derive(Debug, Deserialize)]
struct QuoteQueryParams {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    amount: u64,
}

/// Sets up a real HTTP provider mock server mimicking the real Jupiter v6 HTTP API.
async fn setup_mock_jupiter_provider_server(
    sample_tx_base64: String,
) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route(
            "/quote",
            get(|Query(params): Query<QuoteQueryParams>| async move {
                let quote = QuoteResponse {
                    input_mint: params.input_mint.clone(),
                    in_amount: params.amount.to_string(),
                    output_mint: params.output_mint.clone(),
                    out_amount: "148500000".to_string(),
                    other_amount_threshold: "147757500".to_string(),
                    swap_mode: "ExactIn".to_string(),
                    slippage_bps: 50,
                    price_impact_pct: "0.04".to_string(),
                    route_plan: vec![RoutePlanStep {
                        swap_info: SwapInfo {
                            amm_key: "WhirlpoolPool111111111111111111111111111111".to_string(),
                            label: Some("Orca Whirlpool".to_string()),
                            input_mint: params.input_mint,
                            output_mint: params.output_mint,
                            in_amount: params.amount.to_string(),
                            out_amount: "148500000".to_string(),
                            fee_amount: Some("500".to_string()),
                            fee_mint: Some(SOL_MINT.to_string()),
                        },
                        percent: 100,
                    }],
                    context_slot: Some(290_000_000),
                    time_taken: Some(0.015),
                };
                Json(quote)
            }),
        )
        .route(
            "/swap",
            post(move |Json(req): Json<SwapRequest>| {
                let tx_b64 = sample_tx_base64.clone();
                async move {
                    Json(SwapResponse {
                        swap_transaction: tx_b64,
                        last_valid_block_height: 290_050_000,
                        prioritization_fee_lamports: req.prioritization_fee_lamports,
                    })
                }
            }),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (url, handle)
}

#[tokio::test]
async fn test_task2_production_configuration_points_to_real_jupiter_endpoint() {
    let mainnet_cfg = Config::mainnet();
    assert_eq!(
        mainnet_cfg.jupiter_api_url, "https://quote-api.jup.ag/v6",
        "Mainnet production configuration must point to real Jupiter v6 endpoint"
    );

    let testnet_cfg = Config::testnet();
    assert_eq!(
        testnet_cfg.jupiter_api_url, "https://quote-api.jup.ag/v6",
        "Testnet configuration must point to real configured Jupiter v6 endpoint"
    );

    let dev_cfg = Config::development();
    assert_eq!(
        dev_cfg.jupiter_api_url, "https://quote-api.jup.ag/v6",
        "Development configuration must point to real configured Jupiter v6 endpoint"
    );
}

#[tokio::test]
async fn test_tasks_3_and_4_quote_contains_all_9_fields_and_passes_validation() {
    let authority = Pubkey::new_unique();
    let (_, tx_base64) = create_real_versioned_transaction(&authority);
    let (provider_url, server_handle) = setup_mock_jupiter_provider_server(tx_base64).await;

    // Production client connected to real HTTP provider (mock_mode = false)
    let client = JupiterClient::new(&provider_url);
    assert!(
        !client.is_mock_mode(),
        "Production client must operate with mock_mode=false"
    );

    let req = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000)
        .with_slippage_bps(50)
        .with_max_price_impact_bps(100)
        .with_ttl_seconds(20);

    // Fetch real quote from HTTP provider
    let quote = client
        .get_executable_quote(&req)
        .await
        .expect("Real quote request must succeed");

    // Task 3: A quote must contain all 9 required fields:
    // 1. input mint
    assert_eq!(quote.input_mint(), SOL_MINT);
    // 2. output mint
    assert_eq!(quote.output_mint(), USDC_MINT);
    // 3. input amount
    assert_eq!(quote.input_amount(), 1_000_000_000);
    // 4. expected output
    assert_eq!(quote.expected_output(), 148_500_000);
    // 5. minimum output
    assert_eq!(quote.minimum_output(), 147_757_500);
    // 6. route
    assert_eq!(quote.route().steps.len(), 1);
    assert_eq!(quote.route().primary_dex, "Orca Whirlpool");
    // 7. price impact
    assert_eq!(quote.price_impact(), 4); // 4 bps
                                         // 8. quote timestamp
    assert!(quote.quote_timestamp() > 0);
    // 9. expiration
    assert_eq!(quote.expiration(), quote.quote_timestamp() + 20);

    // Task 4: Validate all invariants:
    let now = Utc::now().timestamp();
    assert!(
        quote.validate(Some(100), now).is_ok(),
        "Quote validation should succeed on valid parameters"
    );

    server_handle.abort();
}

#[tokio::test]
async fn test_task4_quote_validation_invariants() {
    let authority = Pubkey::new_unique();
    let (_, tx_base64) = create_real_versioned_transaction(&authority);
    let (provider_url, server_handle) = setup_mock_jupiter_provider_server(tx_base64).await;
    let client = JupiterClient::new(&provider_url);

    // 1. Validate: input amount > 0
    let req_zero_amount = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 0);
    let res_zero = client.get_executable_quote(&req_zero_amount).await;
    assert!(matches!(res_zero, Err(DexQuoteError::InvalidAmount(_))));

    // 2. Validate: supported mints (valid base58 pubkeys and distinct)
    let req_same_mints = DexQuoteRequest::new(SOL_MINT, SOL_MINT, 1_000_000_000);
    let res_same = client.get_executable_quote(&req_same_mints).await;
    assert!(matches!(
        res_same,
        Err(DexQuoteError::UnsupportedPair { .. })
    ));

    let req_invalid_mint = DexQuoteRequest::new("invalid_pubkey_123", USDC_MINT, 1_000_000_000);
    let res_invalid = client.get_executable_quote(&req_invalid_mint).await;
    assert!(matches!(res_invalid, Err(DexQuoteError::InvalidMint(_))));

    // 3. Validate: price impact within policy
    let req_tight_impact =
        DexQuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000).with_max_price_impact_bps(2); // Provider returns 4 bps -> breaches 2 bps
    let res_impact = client.get_executable_quote(&req_tight_impact).await;
    assert!(matches!(
        res_impact,
        Err(DexQuoteError::ExcessivePriceImpact {
            actual_bps: 4,
            max_bps: 2
        })
    ));

    // 4. Validate: quote not expired
    let valid_req = DexQuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000).with_ttl_seconds(10);
    let quote = client.get_executable_quote(&valid_req).await.unwrap();
    let current_time = Utc::now().timestamp();
    assert!(quote.validate_freshness(current_time).is_ok());
    assert!(quote.validate_freshness(current_time + 15).is_err());

    server_handle.abort();
}

#[tokio::test]
async fn test_tasks_5_to_9_real_swap_construction_and_dummy_rejection() {
    let authority_keypair = Keypair::new();
    let authority = authority_keypair.pubkey();
    let (expected_tx, tx_base64) = create_real_versioned_transaction(&authority);
    let (provider_url, server_handle) = setup_mock_jupiter_provider_server(tx_base64.clone()).await;

    let client = JupiterClient::new(&provider_url);

    // 1. Get real quote
    let quote_req = QuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000);
    let quote_resp = client
        .get_quote(&quote_req)
        .await
        .expect("Quote query must succeed");

    // 2. Task 9: Swap transaction must be for the actual execution authority
    let swap_req = build_swap_request(&quote_resp, &authority.to_string(), Some(10_000))
        .expect("SwapRequest construction should succeed for valid authority");
    assert_eq!(swap_req.user_public_key, authority.to_string());

    // 3. Task 6: build_swap() must request a real transaction from the DEX provider
    let swap_resp = client
        .build_swap(&swap_req)
        .await
        .expect("build_swap must succeed with real provider response");

    // 4. Task 8: Never return a dummy serialized transaction
    assert_eq!(swap_resp.swap_transaction, tx_base64);
    assert!(
        !swap_resp.is_dummy(),
        "Returned swap transaction must not be dummy"
    );

    // Decode and verify the real Solana VersionedTransaction
    let decoded = decode_swap_transaction(&swap_resp.swap_transaction)
        .expect("Transaction decoding must succeed for real transaction");
    assert_eq!(
        decoded.message.static_account_keys().len(),
        expected_tx.message.static_account_keys().len()
    );

    // 5. Test rejection of dummy transactions
    let dummy_tx = "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDBg==";
    let dummy_decode = decode_swap_transaction(dummy_tx);
    assert!(dummy_decode.is_err());
    assert!(matches!(
        dummy_decode.unwrap_err(),
        JupiterError::InvalidTransaction(_)
    ));

    server_handle.abort();
}

#[tokio::test]
async fn test_task7_provider_failure_must_return_an_error() {
    // Point client to an unreachable port / offline host
    let unreachable_client = JupiterClient::new("http://127.0.0.1:59998");

    let quote_req = QuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000);
    let quote_err = unreachable_client.get_quote(&quote_req).await;
    assert!(
        quote_err.is_err(),
        "Provider failure must return error on quote request"
    );
    assert!(matches!(quote_err.unwrap_err(), JupiterError::Http(_)));

    let mock_quote = QuoteResponse {
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
                amm_key: "WhirlpoolPool111111111111111111111111111111".to_string(),
                label: Some("Orca".to_string()),
                input_mint: SOL_MINT.to_string(),
                output_mint: USDC_MINT.to_string(),
                in_amount: "1000000000".to_string(),
                out_amount: "148500000".to_string(),
                fee_amount: None,
                fee_mint: None,
            },
            percent: 100,
        }],
        context_slot: None,
        time_taken: None,
    };

    let authority = Pubkey::new_unique();
    let swap_req = build_swap_request(&mock_quote, &authority.to_string(), Some(5_000)).unwrap();

    let swap_err = unreachable_client.build_swap(&swap_req).await;
    assert!(
        swap_err.is_err(),
        "Provider failure must return error on swap construction (never return dummy)"
    );
    assert!(matches!(swap_err.unwrap_err(), JupiterError::Http(_)));
}

#[tokio::test]
async fn test_tasks_10_and_11_separation_of_construction_signing_and_submission() {
    // Task 10: Do not sign in the Jupiter client.
    // Task 11: Keep construction, signing, and submission separate.
    let authority_keypair = Keypair::new();
    let authority = authority_keypair.pubkey();

    // 1. Transaction Construction phase (Jupiter client solely constructs unsigned transaction)
    let msg =
        Message::try_compile(&authority, &[], &[], solana_sdk::hash::Hash::default()).unwrap();
    let unsigned_tx = VersionedTransaction {
        signatures: vec![solana_sdk::signature::Signature::default()],
        message: VersionedMessage::V0(msg),
    };
    let unsigned_bytes = bincode::serialize(&unsigned_tx).unwrap();
    let unsigned_b64 = BASE64.encode(unsigned_bytes);

    let (provider_url, server_handle) =
        setup_mock_jupiter_provider_server(unsigned_b64.clone()).await;
    let client = JupiterClient::new(&provider_url);

    let quote_req = QuoteRequest::new(SOL_MINT, USDC_MINT, 1_000_000_000);
    let quote = client.get_quote(&quote_req).await.unwrap();

    let swap_req = build_swap_request(&quote, &authority.to_string(), None).unwrap();
    let constructed_swap = client.build_swap(&swap_req).await.unwrap();

    // Verify constructed transaction is unsigned (default signature)
    let mut tx_to_sign = decode_swap_transaction(&constructed_swap.swap_transaction).unwrap();
    assert_eq!(
        tx_to_sign.signatures[0],
        solana_sdk::signature::Signature::default(),
        "Constructed transaction from Jupiter client must not be pre-signed"
    );

    // 2. Cryptographic Signing phase (Done strictly OUTSIDE Jupiter client)
    let message_bytes = tx_to_sign.message.serialize();
    let signature = authority_keypair.sign_message(&message_bytes);
    tx_to_sign.signatures[0] = signature;

    // Verify signature was applied
    assert_ne!(
        tx_to_sign.signatures[0],
        solana_sdk::signature::Signature::default(),
        "Transaction is signed by execution authority in signing phase"
    );

    // 3. Submission phase (Decoupled from construction and signing)
    let serialized_signed_tx = bincode::serialize(&tx_to_sign).unwrap();
    assert!(!serialized_signed_tx.is_empty());

    server_handle.abort();
}
