//! Integration test suite for Phase 11: Deterministic Execution Plan & Idempotency Layer.
//!
//! # Objective
//! Convert an approved allocation decision into a deterministic execution plan.
//!
//! # Core Security Principles Tested
//! The planner must NOT:
//! - Sign transactions
//! - Access private keys
//! - Submit transactions
//! - Bypass risk controls
//! - Modify approved quantities
//!
//! # Verified Requirements
//! - ExecutionPlan contains all 13 canonical fields
//! - Canonical serialization is deterministic: two identical plans produce identical canonical bytes
//! - Replay protection & idempotency collision detection
//! - Strict quantity preservation from authorized decisions

use chrono::{DateTime, TimeZone, Utc};
use equity_catalyst_api::{
    engines::execution_planner::{
        ExecutionPlan, ExecutionPlanner, IdempotencyStatus, IdempotencyTracker,
        OracleReferenceInfo, PlannerError,
    },
    models::{CanonicalAssetModel, PolicyModel, PortfolioModel},
    AllocationProposal, DeterministicPolicyAuthorizer, ExecutionAuthorization, MarketPriceUpdate,
    ValidationContext,
};
use equity_catalyst_jupiter::{DexQuote, DexRouteInfo, DexRouteStep};
use equity_catalyst_shared::{
    asset::{
        Asset, AssetIdentity, AssetProvider, AssetStatus, AssetType, Network, ProviderConfig,
        TokenDetails,
    },
    shariah::{
        DenominatorMethod, RegisteredAsset, ScreeningStandard, ShariahEligibility, ShariahStatus,
    },
    AssetApprovalStatus,
};
use std::sync::Arc;
use uuid::Uuid;

const VAULT_ADDR: &str = "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4";
const AAPL_MINT: &str = "AAPL111111111111111111111111111111111111111";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const FEED_AAPL: &str = "feed1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

/// Test helper building an authorized decision from Phase 10 engine
fn build_authorized_decision(
    is_buy: bool,
    eval_time: DateTime<Utc>,
) -> (ExecutionAuthorization, DexQuote) {
    let now = eval_time;

    let policy = PolicyModel {
        policy_address: "pol-001".to_string(),
        vault_address: VAULT_ADDR.to_string(),
        authority: "auth-001".to_string(),
        min_cash_bps: 1000,
        max_position_bps: 3000,
        stop_loss_bps: 800,
        take_profit_bps: 2000,
        rebalance_threshold_bps: 150,
        is_active: true,
        bump: 255,
        created_at: now,
        updated_at: now,
    };

    let positions = vec![PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: VAULT_ADDR.to_string(),
        asset_symbol: "AAPL".to_string(),
        asset_mint: AAPL_MINT.to_string(),
        amount: 50_000_000,
        entry_price_usd: 190.0,
        current_price_usd: 200.0,
        current_value_usd: 10_000.0,
        target_weight_bps: 1500,
        current_weight_bps: 1000,
        last_rebalanced_at: Some(now),
        updated_at: now,
    }];

    let canonical_asset = CanonicalAssetModel {
        asset_id: "backed:AAPL".to_string(),
        symbol: "AAPL".to_string(),
        mint_address: AAPL_MINT.to_string(),
        legal_issuer: "Backed Finance AG".to_string(),
        custodian: "Incore Bank AG".to_string(),
        underlying_asset_identifier: "US0378331005".to_string(),
        is_active: true,
        approval_status: AssetApprovalStatus::ShariahApproved,
        decimals: 6,
        created_at: now,
        updated_at: now,
    };

    let asset = Asset {
        identity: AssetIdentity {
            asset_id: "backed:AAPL".to_string(),
            symbol: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            asset_type: AssetType::Stock,
            underlying_reference: "US0378331005".to_string(),
        },
        token: TokenDetails {
            mint: AAPL_MINT.to_string(),
            decimals: 6,
            network: Network::SolanaMainnet,
        },
        provider: ProviderConfig {
            provider: AssetProvider::Backed,
            price_feed_id: FEED_AAPL.to_string(),
            meteora_pool: None,
            secondary_reference: None,
            status: AssetStatus::Active,
        },
    };

    let eligibility = ShariahEligibility {
        status: ShariahStatus::Approved,
        standard: ScreeningStandard::Aaoifi21,
        business_activity_approved: true,
        debt_ratio_bps: 1250,
        interest_bearing_cash_bps: 800,
        receivables_cash_bps: Some(400),
        impure_income_bps: 50,
        denominator_method: DenominatorMethod::CurrentMarketCap,
        ownership_verified: true,
        evidence_hash: "proof_hash_123".to_string(),
        reviewed_at: now.timestamp() - 3600,
        expires_at: now.timestamp() + 86400 * 30,
        policy_version: "v1.0".to_string(),
    };

    let shariah_record = RegisteredAsset { asset, eligibility };

    let mut oracle_price = MarketPriceUpdate::from_raw(
        "backed:AAPL",
        "AAPL",
        AAPL_MINT,
        FEED_AAPL,
        200_00000000,
        -8,
        2000000,
        now.timestamp() - 5,
        30,
        Some(100),
    )
    .unwrap();
    oracle_price.is_stale = false;

    let dex_quote = if is_buy {
        DexQuote {
            quote_id: "quote-buy-001".to_string(),
            provider_id: "jupiter".to_string(),
            input_mint: USDC_MINT.to_string(),
            output_mint: AAPL_MINT.to_string(),
            input_amount: 5_000_000_000,        // 5,000 USDC
            expected_output_amount: 25_000_000, // 25 AAPL
            minimum_output_amount: 24_875_000,  // 50 bps slippage
            price_impact_bps: 10,
            price_impact_pct: "0.10%".to_string(),
            effective_rate: 0.005,
            quote_timestamp: now.timestamp() - 2,
            expires_at: now.timestamp() + 30,
            ttl_seconds: 45,
            route_info: DexRouteInfo {
                steps: vec![DexRouteStep {
                    dex_label: "Orca Whirlpool".to_string(),
                    amm_key: "Pool1".to_string(),
                    input_mint: USDC_MINT.to_string(),
                    output_mint: AAPL_MINT.to_string(),
                    in_amount: 5_000_000_000,
                    out_amount: 25_000_000,
                    percent: 100,
                }],
                num_hops: 1,
                primary_dex: "Orca Whirlpool".to_string(),
            },
            raw_payload: None,
        }
    } else {
        DexQuote {
            quote_id: "quote-sell-001".to_string(),
            provider_id: "jupiter".to_string(),
            input_mint: AAPL_MINT.to_string(),
            output_mint: USDC_MINT.to_string(),
            input_amount: 25_000_000,              // 25 AAPL
            expected_output_amount: 5_000_000_000, // 5,000 USDC
            minimum_output_amount: 4_975_000_000,  // 50 bps slippage
            price_impact_bps: 10,
            price_impact_pct: "0.10%".to_string(),
            effective_rate: 200.0,
            quote_timestamp: now.timestamp() - 2,
            expires_at: now.timestamp() + 30,
            ttl_seconds: 45,
            route_info: DexRouteInfo {
                steps: vec![DexRouteStep {
                    dex_label: "Orca Whirlpool".to_string(),
                    amm_key: "Pool1".to_string(),
                    input_mint: AAPL_MINT.to_string(),
                    output_mint: USDC_MINT.to_string(),
                    in_amount: 25_000_000,
                    out_amount: 5_000_000_000,
                    percent: 100,
                }],
                num_hops: 1,
                primary_dex: "Orca Whirlpool".to_string(),
            },
            raw_payload: None,
        }
    };

    let proposal = AllocationProposal {
        proposal_id: Uuid::new_v4(),
        vault_address: VAULT_ADDR.to_string(),
        asset_id: "backed:AAPL".to_string(),
        symbol: "AAPL".to_string(),
        mint_address: AAPL_MINT.to_string(),
        is_buy,
        target_amount: 25_000_000,
        proposed_usd_value: 5_000,
        max_slippage_bps: 50,
        proposed_at: now,
        ai_confidence: Some(0.95),
        ai_rationale: Some("Target allocation".to_string()),
        ai_approved: false,
    };

    let context = ValidationContext {
        policy: &policy,
        positions: &positions,
        total_portfolio_usd: 100_000,
        available_cash_usd: 40_000,
        canonical_asset: Some(&canonical_asset),
        shariah_record: Some(&shariah_record),
        shariah_registry: None,
        oracle_price: Some(&oracle_price),
        dex_quote: Some(&dex_quote),
        evaluation_time: now,
        max_trade_size_usd: None,
        max_oracle_conf_bps: None,
        max_allowed_slippage_bps: None,
        max_allowed_price_impact_bps: None,
        max_oracle_staleness_secs: None,
    };

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&proposal, &context);
    let authorization = outcome
        .authorization()
        .expect("Must authorize baseline proposal")
        .clone();

    (authorization, dex_quote)
}

fn sample_execution_plan() -> ExecutionPlan {
    let plan_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
    let decision_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();

    ExecutionPlan {
        plan_id,
        policy_decision_id: decision_id,
        idempotency_key: "idemp:22222222-2222-2222-2222-222222222222:nonce-123".to_string(),
        vault_address: VAULT_ADDR.to_string(),
        asset_id: "backed:AAPL".to_string(),
        symbol: "AAPL".to_string(),
        is_buy: true,
        input_mint: USDC_MINT.to_string(),
        output_mint: AAPL_MINT.to_string(),
        input_amount: 5_000_000_000,
        expected_output_amount: 25_000_000,
        minimum_output_amount: 24_875_000,
        slippage_bps: 50,
        quote_id: "quote-jupiter-999".to_string(),
        quote_timestamp: 1_750_000_000,
        oracle_reference: OracleReferenceInfo {
            price_scaled: 200_000_000,
            publish_time: 1_749_999_995,
            conf_bps: 10,
            feed_id: FEED_AAPL.to_string(),
        },
        expires_at: 1_750_000_030,
        created_at: 1_750_000_000,
    }
}

#[test]
fn test_plan_contains_all_13_canonical_requirements() {
    let plan = sample_execution_plan();

    // 1. Asset identity
    assert_eq!(plan.asset_identity(), "backed:AAPL");
    assert_eq!(plan.symbol, "AAPL");

    // 2. Input mint
    assert_eq!(plan.input_mint(), USDC_MINT);

    // 3. Output mint
    assert_eq!(plan.output_mint(), AAPL_MINT);

    // 4. Input amount
    assert_eq!(plan.input_amount(), 5_000_000_000);

    // 5. Expected output
    assert_eq!(plan.expected_output(), 25_000_000);

    // 6. Minimum output
    assert_eq!(plan.minimum_output(), 24_875_000);

    // 7. Slippage limit
    assert_eq!(plan.slippage_limit(), 50);

    // 8. Quote ID
    assert_eq!(plan.quote_id(), "quote-jupiter-999");

    // 9. Quote timestamp
    assert_eq!(plan.quote_timestamp(), 1_750_000_000);

    // 10. Oracle reference used for validation
    let oracle = plan.oracle_reference();
    assert_eq!(oracle.price_scaled, 200_000_000);
    assert_eq!(oracle.publish_time, 1_749_999_995);
    assert_eq!(oracle.conf_bps, 10);
    assert_eq!(oracle.feed_id, FEED_AAPL);

    // 11. Policy decision ID
    assert_eq!(
        plan.policy_decision_id(),
        Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()
    );

    // 12. Expiry
    assert_eq!(plan.expiry(), 1_750_000_030);

    // 13. Nonce / idempotency identifier
    assert_eq!(
        plan.nonce(),
        "idemp:22222222-2222-2222-2222-222222222222:nonce-123"
    );
}

#[test]
fn test_canonical_serialization_two_identical_plans_produce_identical_bytes() {
    let plan_1 = sample_execution_plan();
    let plan_2 = sample_execution_plan();

    let bytes_1 = plan_1.to_canonical_bytes();
    let bytes_2 = plan_2.to_canonical_bytes();

    assert_eq!(
        bytes_1, bytes_2,
        "Two identical plans must produce identical canonical bytes"
    );

    // Cryptographic digests must also be bit-identical
    assert_eq!(plan_1.canonical_digest(), plan_2.canonical_digest());
    assert_eq!(plan_1.canonical_digest_hex(), plan_2.canonical_digest_hex());
}

#[test]
fn test_canonical_serialization_distinct_plans_produce_different_bytes() {
    let baseline = sample_execution_plan();
    let baseline_bytes = baseline.to_canonical_bytes();
    let baseline_digest = baseline.canonical_digest_hex();

    // 1. Alter input amount by 1 unit
    let mut modified = baseline.clone();
    modified.input_amount += 1;
    assert_ne!(baseline_bytes, modified.to_canonical_bytes());
    assert_ne!(baseline_digest, modified.canonical_digest_hex());

    // 2. Alter minimum output amount
    let mut modified = baseline.clone();
    modified.minimum_output_amount -= 1;
    assert_ne!(baseline_bytes, modified.to_canonical_bytes());
    assert_ne!(baseline_digest, modified.canonical_digest_hex());

    // 3. Alter slippage limit
    let mut modified = baseline.clone();
    modified.slippage_bps += 1;
    assert_ne!(baseline_bytes, modified.to_canonical_bytes());
    assert_ne!(baseline_digest, modified.canonical_digest_hex());

    // 4. Alter oracle reference price
    let mut modified = baseline.clone();
    modified.oracle_reference.price_scaled += 1_000;
    assert_ne!(baseline_bytes, modified.to_canonical_bytes());
    assert_ne!(baseline_digest, modified.canonical_digest_hex());

    // 5. Alter idempotency nonce
    let mut modified = baseline.clone();
    modified.idempotency_key = "idemp:diff-nonce".to_string();
    assert_ne!(baseline_bytes, modified.to_canonical_bytes());
    assert_ne!(baseline_digest, modified.canonical_digest_hex());
}

#[tokio::test]
async fn test_planner_converts_authorized_decision_cleanly() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, quote) = build_authorized_decision(true, now);

    let planner = ExecutionPlanner::new();
    let plan = planner
        .plan_execution(
            &auth,
            &quote,
            FEED_AAPL,
            10,
            Some("client-nonce-001"),
            now.timestamp(),
        )
        .await
        .expect("Must plan execution successfully");

    assert_eq!(plan.policy_decision_id, auth.authorization_id);
    assert_eq!(plan.asset_id, auth.asset_id);
    assert_eq!(plan.symbol, "AAPL");
    assert_eq!(plan.vault_address, auth.vault_address);
    assert_eq!(plan.input_mint, quote.input_mint);
    assert_eq!(plan.output_mint, quote.output_mint);
    assert_eq!(plan.input_amount, quote.input_amount);
    assert_eq!(plan.expected_output_amount, quote.expected_output_amount);
    assert_eq!(plan.minimum_output_amount, quote.minimum_output_amount);
    assert_eq!(plan.slippage_bps, auth.max_slippage_bps);
    assert_eq!(plan.quote_id, quote.quote_id);
    assert_eq!(
        plan.oracle_reference.price_scaled,
        auth.oracle_reference_price_scaled
    );
    assert_eq!(plan.oracle_reference.feed_id, FEED_AAPL);
    assert_eq!(plan.expires_at, quote.expires_at);

    // Verify temporal validity helpers
    assert!(!plan.is_expired(now.timestamp()));
    assert!(plan.is_valid_at(now.timestamp()));
    assert!(plan.is_expired(quote.expires_at + 1));
}

#[tokio::test]
async fn test_planner_refuses_modified_quantities_sell_trade() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, mut quote) = build_authorized_decision(false, now);

    let planner = ExecutionPlanner::new();

    // Adversarial modification: quote input amount altered from authorized amount (25_000_000) to 20_000_000
    quote.input_amount = 20_000_000;

    let result = planner
        .plan_execution(&auth, &quote, FEED_AAPL, 10, None, now.timestamp())
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        PlannerError::QuantityMismatch { expected, got, .. } => {
            assert_eq!(expected, auth.authorized_amount);
            assert_eq!(got, 20_000_000);
        }
        other => panic!("Expected QuantityMismatch, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_planner_refuses_modified_quantities_minimum_output() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, mut quote) = build_authorized_decision(true, now);

    let planner = ExecutionPlanner::new();

    // Adversarial modification: quote minimum output reduced below authorized minimum output
    quote.minimum_output_amount = auth.dex_minimum_output_amount - 10_000;

    let result = planner
        .plan_execution(&auth, &quote, FEED_AAPL, 10, None, now.timestamp())
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        PlannerError::QuantityMismatch { expected, got, .. } => {
            assert_eq!(expected, auth.dex_minimum_output_amount);
            assert_eq!(got, quote.minimum_output_amount);
        }
        other => panic!("Expected QuantityMismatch, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_planner_refuses_expired_quote() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, mut quote) = build_authorized_decision(true, now);

    let planner = ExecutionPlanner::new();

    // Quote expired 5s ago
    quote.expires_at = now.timestamp() - 5;

    let result = planner
        .plan_execution(&auth, &quote, FEED_AAPL, 10, None, now.timestamp())
        .await;

    assert!(matches!(result, Err(PlannerError::QuoteExpired { .. })));
}

#[tokio::test]
async fn test_planner_refuses_expired_authorization() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, quote) = build_authorized_decision(true, now);

    let planner = ExecutionPlanner::new();

    // Current time is after authorization validity
    let late_time = auth.valid_until.timestamp() + 10;

    let result = planner
        .plan_execution(&auth, &quote, FEED_AAPL, 10, None, late_time)
        .await;

    assert!(matches!(
        result,
        Err(PlannerError::AuthorizationExpired { .. })
    ));
}

#[tokio::test]
async fn test_planner_refuses_mint_mismatch() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, mut quote) = build_authorized_decision(true, now);

    let planner = ExecutionPlanner::new();

    // Alter output mint
    quote.output_mint = "WRONG_MINT_11111111111111111111111111111111".to_string();

    let result = planner
        .plan_execution(&auth, &quote, FEED_AAPL, 10, None, now.timestamp())
        .await;

    assert!(matches!(result, Err(PlannerError::MintMismatch { .. })));
}

#[tokio::test]
async fn test_planner_refuses_excessive_slippage() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, mut quote) = build_authorized_decision(true, now);

    let planner = ExecutionPlanner::new();

    // Authorized slippage is 50 bps. Quote has price impact of 80 bps.
    quote.price_impact_bps = 80;

    let result = planner
        .plan_execution(&auth, &quote, FEED_AAPL, 10, None, now.timestamp())
        .await;

    assert!(matches!(
        result,
        Err(PlannerError::SlippageToleranceExceeded {
            quote_bps: 80,
            max_allowed_bps: 50
        })
    ));
}

#[tokio::test]
async fn test_idempotency_duplicate_plan_detection_and_replay() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, quote) = build_authorized_decision(true, now);

    let planner = ExecutionPlanner::new();

    // 1. Initial plan creation
    let plan_1 = planner
        .plan_execution(
            &auth,
            &quote,
            FEED_AAPL,
            10,
            Some("idemp-unique-1"),
            now.timestamp(),
        )
        .await
        .expect("Initial plan must succeed");

    // 2. Exact same plan replay with identical parameters and nonce
    let plan_2 = planner
        .plan_execution(
            &auth,
            &quote,
            FEED_AAPL,
            10,
            Some("idemp-unique-1"),
            now.timestamp(),
        )
        .await
        .expect("Idempotent replay must succeed");

    assert_eq!(plan_1.to_canonical_bytes(), plan_2.to_canonical_bytes());
    assert_eq!(plan_1.canonical_digest(), plan_2.canonical_digest());
}

#[tokio::test]
async fn test_idempotency_conflict_rejection() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, quote_1) = build_authorized_decision(true, now);

    let planner = ExecutionPlanner::new();

    // Plan with quote 1 under nonce "fixed-nonce"
    planner
        .plan_execution(
            &auth,
            &quote_1,
            FEED_AAPL,
            10,
            Some("fixed-nonce"),
            now.timestamp(),
        )
        .await
        .expect("First plan succeeds");

    // Now attempt to use the SAME nonce "fixed-nonce" with a DIFFERENT quote
    let mut quote_2 = quote_1.clone();
    quote_2.quote_id = "quote-jupiter-diff".to_string();
    quote_2.expected_output_amount += 1_000;

    let result = planner
        .plan_execution(
            &auth,
            &quote_2,
            FEED_AAPL,
            10,
            Some("fixed-nonce"),
            now.timestamp(),
        )
        .await;

    assert!(
        matches!(result, Err(PlannerError::IdempotencyConflict { .. })),
        "Reusing an idempotency key with different plan payload must be rejected"
    );
}

#[tokio::test]
async fn test_idempotency_decision_replay_protection() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, quote) = build_authorized_decision(true, now);

    let planner = ExecutionPlanner::new();

    // Plan with nonce A
    planner
        .plan_execution(
            &auth,
            &quote,
            FEED_AAPL,
            10,
            Some("nonce-a"),
            now.timestamp(),
        )
        .await
        .expect("First plan succeeds");

    // Attempt to plan the SAME decision ID under a DIFFERENT nonce B
    let result = planner
        .plan_execution(
            &auth,
            &quote,
            FEED_AAPL,
            10,
            Some("nonce-b"),
            now.timestamp(),
        )
        .await;

    assert!(
        matches!(result, Err(PlannerError::DuplicateExecutionPlan { .. })),
        "Planning the same policy decision under a different nonce must be rejected as duplicate"
    );
}

#[tokio::test]
async fn test_idempotency_lifecycle_status_updates() {
    let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();
    let (auth, quote) = build_authorized_decision(true, now);

    let tracker = Arc::new(IdempotencyTracker::new());
    let planner = ExecutionPlanner::with_tracker(tracker.clone());

    let plan = planner
        .plan_execution(
            &auth,
            &quote,
            FEED_AAPL,
            10,
            Some("lifecycle-nonce"),
            now.timestamp(),
        )
        .await
        .unwrap();

    let key = plan.idempotency_key;

    // Verify initial status is Planned
    let record = tracker.get_record(&key).await.unwrap();
    assert_eq!(record.status, IdempotencyStatus::Planned);

    // Update to Executing
    tracker
        .update_status(&key, IdempotencyStatus::Executing)
        .await
        .unwrap();
    assert_eq!(
        tracker.get_record(&key).await.unwrap().status,
        IdempotencyStatus::Executing
    );

    // Update to Completed
    tracker
        .update_status(&key, IdempotencyStatus::Completed)
        .await
        .unwrap();
    assert_eq!(
        tracker.get_record(&key).await.unwrap().status,
        IdempotencyStatus::Completed
    );
}

#[test]
fn test_plan_json_serialization_roundtrip() {
    let plan = sample_execution_plan();

    let json = serde_json::to_string(&plan).expect("Must serialize to JSON");
    let deserialized: ExecutionPlan =
        serde_json::from_str(&json).expect("Must deserialize from JSON");

    assert_eq!(plan, deserialized);
    assert_eq!(plan.to_canonical_bytes(), deserialized.to_canonical_bytes());
}
