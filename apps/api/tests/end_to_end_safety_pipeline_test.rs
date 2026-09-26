//! End-to-End Integration Tests for the Complete Market-Data-to-Execution Safety Pipeline (Phase 17).
//!
//! # Objective
//! Create comprehensive end-to-end integration tests validating the complete 18-step pipeline
//! from canonical asset registration to execution record storage, adhering strictly to all
//! safety boundaries.
//!
//! # 18-Step Pipeline Flow:
//! 1. Register asset (`CanonicalAssetRepository`)
//! 2. Approve asset (transition to `ShariahApproved`)
//! 3. Create asset-to-Pyth mapping (`AssetMarketDataRepository`)
//! 4. Activate asset (`activate_asset`)
//! 5. Subscription watcher detects asset (`AssetSubscriptionWatcher::poll_once`)
//! 6. Pyth stream subscribes (`current_subscription_set()`)
//! 7. Pyth price arrives (`MarketPriceUpdate::from_raw`)
//! 8. Oracle validates price (freshness, confidence bounds, feed identity)
//! 9. MarketDataStore updates (`MarketDataStore::update`)
//! 10. DEX quote is obtained (`MockDexQuoter::get_executable_quote`)
//! 11. Allocation proposal is created (`AllocationProposal`)
//! 12. Deterministic policy validates proposal (`DeterministicPolicyAuthorizer::authorize`)
//! 13. ExecutionPlan is generated (`ExecutionPlanner::plan_execution`)
//! 14. External signer signs (`TransactionSignerService::sign` via `ExternalSigner`)
//! 15. Transaction is submitted (wire serialized, broadcast simulated)
//! 16. Anchor program executes DEX CPI (`AnchorClient::build_execute_action_ix`, pre-balance snapshot)
//! 17. Actual output is measured (post-balance reload, delta derivation, non-derivation checks)
//! 18. Execution record is stored (`ExecutionBalanceTracker::complete`, stored with 7 canonical fields)
//!
//! # Failure Scenarios Verified (Failing Closed):
//! * Asset deactivated before execution
//! * Stale oracle
//! * Wrong feed
//! * Expired quote
//! * Excessive slippage
//! * Signer unavailable
//! * Duplicate execution
//! * Failed transaction (negative balance delta and submission failure)

use chrono::Utc;
use equity_catalyst_api::{
    engines::{
        execution_planner::{ExecutionPlan, ExecutionPlanner, IdempotencyTracker, PlannerError},
        execution_recorder::{ExecutionBalanceTracker, ExecutionRecord, ExecutionRecorderError},
        execution_signer::{
            DevTestSigner, ExternalSigner, SignerError, TransactionBuilder,
            TransactionSignerService, UnavailableSigner,
        },
        pipeline_safety::{PipelineExecutionContext, PipelineFailure, TradingPipelineSafetyGuard},
        policy_engine::authorization::{
            AllocationProposal, DeterministicPolicyAuthorizer, ExecutionAuthorization,
            PolicyRejectionReason,
        },
    },
    models::{
        canonical_asset::CreateAssetRequest, CanonicalAssetModel, PolicyModel, PortfolioModel,
    },
    repositories::{
        asset_market_data_repository::AssetMarketDataRepository,
        canonical_asset_repository::CanonicalAssetRepository,
    },
    services::{
        asset_subscription_watcher::{AssetSubscriptionWatcher, SubscriptionWatcherConfig},
        MarketDataStore, MarketPriceUpdate, PriceFreshness,
    },
    ValidationContext,
};
use equity_catalyst_jupiter::{DexQuote, DexQuoteRequest, DexQuoter, DexRouteInfo, MockDexQuoter};
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
use equity_catalyst_solana::{
    accounts::{find_compliance_pda, program_id},
    anchor_client::AnchorClient,
};
use solana_sdk::{hash::Hash, pubkey::Pubkey};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use uuid::Uuid;

// Canonical test constants
const VAULT_ADDR: &str = "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4";
const AAPL_ASSET_ID: &str = "backed:AAPLx";
const AAPL_SYMBOL: &str = "AAPL";
const AAPL_MINT: &str = "AAPL111111111111111111111111111111111111111";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const AAPL_FEED: &str = "2554432244243455243452452452345234523452345234523452345234523452";
const WRONG_FEED: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

/// Helper constructing a Shariah compliance record for AAPL
fn create_test_shariah_record(now_ts: i64) -> RegisteredAsset {
    RegisteredAsset {
        asset: Asset {
            identity: AssetIdentity {
                asset_id: AAPL_ASSET_ID.to_string(),
                symbol: AAPL_SYMBOL.to_string(),
                name: "Apple Inc.".to_string(),
                asset_type: AssetType::Stock,
                underlying_reference: "US0378331005".to_string(),
            },
            token: TokenDetails {
                mint: AAPL_MINT.to_string(),
                decimals: 8,
                network: Network::SolanaMainnet,
            },
            provider: ProviderConfig {
                provider: AssetProvider::Backed,
                price_feed_id: AAPL_FEED.to_string(),
                meteora_pool: None,
                secondary_reference: None,
                status: AssetStatus::Active,
            },
        },
        eligibility: ShariahEligibility {
            status: ShariahStatus::Approved,
            standard: ScreeningStandard::Aaoifi21,
            business_activity_approved: true,
            debt_ratio_bps: 1250,           // 12.50% (< 33%)
            interest_bearing_cash_bps: 800, // 8.00% (< 33%)
            receivables_cash_bps: Some(400),
            impure_income_bps: 50, // 0.50% (< 5%)
            denominator_method: DenominatorMethod::CurrentMarketCap,
            ownership_verified: true,
            evidence_hash: "a4c8e2f1837bc21".to_string(),
            reviewed_at: now_ts - 3600,
            expires_at: now_ts + 86400 * 30, // 30 days remaining
            policy_version: "v1.0".to_string(),
        },
    }
}

/// Helper constructing vault policy model
fn create_test_policy(now: chrono::DateTime<Utc>) -> PolicyModel {
    PolicyModel {
        policy_address: "pol-test-vault-001".to_string(),
        vault_address: VAULT_ADDR.to_string(),
        authority: "auth-keeper-001".to_string(),
        min_cash_bps: 1000,     // 10.00% minimum cash reserve
        max_position_bps: 3000, // 30.00% max single position
        stop_loss_bps: 800,
        take_profit_bps: 2000,
        rebalance_threshold_bps: 150,
        is_active: true,
        bump: 255,
        created_at: now,
        updated_at: now,
    }
}

// =============================================================================
// TEST FLOW: FULL 18-STEP END-TO-END PIPELINE SUCCESS
// =============================================================================

#[tokio::test]
async fn test_end_to_end_full_pipeline_success() {
    let now_chrono = Utc::now();
    let now_ts = now_chrono.timestamp();

    // -------------------------------------------------------------------------
    // STEP 1: Register asset
    // -------------------------------------------------------------------------
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let create_req = CreateAssetRequest {
        asset_id: AAPL_ASSET_ID.to_string(),
        symbol: AAPL_SYMBOL.to_string(),
        mint_address: AAPL_MINT.to_string(),
        legal_issuer: "Backed Finance AG".to_string(),
        custodian: "Incore Bank AG".to_string(),
        underlying_asset_identifier: "NASDAQ:AAPL (ISIN US0378331005)".to_string(),
        is_active: false,
        approval_status: Some(AssetApprovalStatus::Pending),
        decimals: 8,
    };
    let registered_asset = asset_repo
        .create_asset(&create_req)
        .await
        .expect("Step 1: Failed to register asset");
    assert_eq!(registered_asset.asset_id, AAPL_ASSET_ID);
    assert_eq!(
        registered_asset.approval_status,
        AssetApprovalStatus::Pending
    );
    assert!(!registered_asset.is_active);

    // -------------------------------------------------------------------------
    // STEP 2: Approve asset
    // -------------------------------------------------------------------------
    asset_repo
        .update_approval_status(AAPL_ASSET_ID, AssetApprovalStatus::Validated)
        .await
        .expect("Step 2a: Failed to transition asset to Validated");

    let approved_asset = asset_repo
        .update_approval_status(AAPL_ASSET_ID, AssetApprovalStatus::ShariahApproved)
        .await
        .expect("Step 2b: Failed to transition asset to ShariahApproved");
    assert_eq!(
        approved_asset.approval_status,
        AssetApprovalStatus::ShariahApproved
    );

    // -------------------------------------------------------------------------
    // STEP 3: Create asset-to-Pyth mapping
    // -------------------------------------------------------------------------
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());
    let mapping = market_repo
        .create_mapping(AAPL_ASSET_ID, AAPL_FEED)
        .await
        .expect("Step 3: Failed to create asset-to-Pyth mapping");
    assert_eq!(mapping.asset_id, AAPL_ASSET_ID);
    assert_eq!(mapping.pyth_feed_id, AAPL_FEED);

    // -------------------------------------------------------------------------
    // STEP 4: Activate asset
    // -------------------------------------------------------------------------
    let activated_asset = asset_repo
        .activate_asset(AAPL_ASSET_ID)
        .await
        .expect("Step 4: Failed to activate asset");
    assert!(activated_asset.is_active);

    // -------------------------------------------------------------------------
    // STEP 5: Subscription watcher detects asset
    // -------------------------------------------------------------------------
    let watcher_config = SubscriptionWatcherConfig::new(Duration::from_millis(50));
    let watcher = AssetSubscriptionWatcher::new(market_repo.clone(), watcher_config);
    let changed = watcher
        .poll_once()
        .await
        .expect("Step 5: Subscription watcher poll failed");
    assert!(
        changed,
        "Step 5: Watcher must detect change when active mapped asset is registered"
    );

    // -------------------------------------------------------------------------
    // STEP 6: Pyth stream subscribes
    // -------------------------------------------------------------------------
    let active_subs = watcher.current_subscription_set();
    assert_eq!(active_subs.len(), 1);
    assert!(
        active_subs.contains_feed(AAPL_FEED),
        "Step 6: Pyth stream must subscribe to mapped feed"
    );
    let sub = active_subs
        .get_by_feed(AAPL_FEED)
        .expect("Step 6: Subscription must be findable by feed");
    assert_eq!(sub.asset_id, AAPL_ASSET_ID);
    assert_eq!(sub.symbol, AAPL_SYMBOL);

    // -------------------------------------------------------------------------
    // STEP 7: Pyth price arrives
    // -------------------------------------------------------------------------
    // Incoming Pyth price: $200.00 (20000000000 with expo -8), conf = 20000000 (0.20 USD = 10 bps)
    let arrival_time = now_ts - 2; // 2 seconds old
    let pyth_price = MarketPriceUpdate::from_raw(
        AAPL_ASSET_ID,
        AAPL_SYMBOL,
        AAPL_MINT,
        AAPL_FEED,
        20_000_000_000, // $200.00
        -8,
        20_000_000, // 0.20 USD confidence interval
        arrival_time,
        30, // 30s staleness limit
        Some(100),
    )
    .expect("Step 7: Valid price payload construction");

    // -------------------------------------------------------------------------
    // STEP 8: Oracle validates price
    // -------------------------------------------------------------------------
    assert_eq!(
        pyth_price.freshness_status(30),
        PriceFreshness::Fresh,
        "Step 8: Oracle must validate price is fresh"
    );
    assert!(
        !pyth_price.is_stale,
        "Step 8: Freshness flag must be false for staleness"
    );
    assert_eq!(
        pyth_price.pyth_feed_id, AAPL_FEED,
        "Step 8: Feed ID must match registered feed"
    );
    let conf_bps = pyth_price.conf_bps;
    assert!(
        conf_bps <= 50,
        "Step 8: Confidence interval ({} bps) must be within risk bounds",
        conf_bps
    );

    // -------------------------------------------------------------------------
    // STEP 9: MarketDataStore updates
    // -------------------------------------------------------------------------
    let market_store = MarketDataStore::new();
    market_store
        .update(pyth_price.clone())
        .await
        .expect("Step 9: Failed to update MarketDataStore");

    let stored_price = market_store
        .get_by_asset_id(AAPL_ASSET_ID)
        .await
        .expect("Step 9: Asset price must be present in MarketDataStore");
    assert_eq!(stored_price.asset_id, AAPL_ASSET_ID);
    assert_eq!(stored_price.symbol, AAPL_SYMBOL);
    assert_eq!(stored_price.mint_address, AAPL_MINT);
    // Integer-scaled micro-USD (6 decimals): 200.00 USD -> 200_000_000
    assert_eq!(stored_price.price_scaled, 200_000_000);

    // Also verify secondary lookups
    assert!(market_store.get_by_mint(AAPL_MINT).await.is_some());
    assert!(market_store.get_by_symbol(AAPL_SYMBOL).await.is_some());
    assert!(market_store.get_by_feed(AAPL_FEED).await.is_some());

    // -------------------------------------------------------------------------
    // STEP 10: DEX quote is obtained
    // -------------------------------------------------------------------------
    let quoter = MockDexQuoter::new("jupiter-v6");
    // Conversion rate: 1 USDC (atomic: 1_000_000) buys 0.005 AAPL (atomic: 500_000)
    // For 5,000 USDC (5_000_000_000 units), rate 0.005 -> 25_000_000 atomic units AAPL
    quoter.add_pair(USDC_MINT, AAPL_MINT, 0.005, 12); // 12 bps price impact
    quoter.set_fixed_timestamp(Some(now_ts - 1));
    quoter.set_custom_ttl(Some(60));

    let quote_req = DexQuoteRequest::new(USDC_MINT, AAPL_MINT, 5_000_000_000)
        .with_slippage_bps(50)
        .with_ttl_seconds(60);
    let dex_quote = quoter
        .get_executable_quote(&quote_req)
        .await
        .expect("Step 10: Failed to obtain DEX quote");

    assert_eq!(dex_quote.input_mint, USDC_MINT);
    assert_eq!(dex_quote.output_mint, AAPL_MINT);
    assert_eq!(dex_quote.input_amount, 5_000_000_000);
    assert_eq!(dex_quote.expected_output_amount, 25_000_000);
    assert_eq!(dex_quote.minimum_output_amount, 24_875_000); // 50 bps slippage limit
    assert_eq!(dex_quote.price_impact_bps, 12);
    assert!(!dex_quote.is_expired(now_ts));

    // -------------------------------------------------------------------------
    // STEP 11: Allocation proposal is created
    // -------------------------------------------------------------------------
    let proposal_id = Uuid::new_v4();
    let proposal = AllocationProposal {
        proposal_id,
        vault_address: VAULT_ADDR.to_string(),
        asset_id: AAPL_ASSET_ID.to_string(),
        symbol: AAPL_SYMBOL.to_string(),
        mint_address: AAPL_MINT.to_string(),
        is_buy: true,
        target_amount: 25_000_000,
        proposed_usd_value: 5_000,
        max_slippage_bps: 50,
        proposed_at: now_chrono,
        ai_confidence: Some(0.97),
        ai_rationale: Some("Target weight alignment within AAOIFI bounds".to_string()),
        ai_approved: false, // Invariant: AI approval boolean is untrusted
    };

    // -------------------------------------------------------------------------
    // STEP 12: Deterministic policy validates proposal
    // -------------------------------------------------------------------------
    let policy = create_test_policy(now_chrono);
    let positions = vec![PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: VAULT_ADDR.to_string(),
        asset_symbol: AAPL_SYMBOL.to_string(),
        asset_mint: AAPL_MINT.to_string(),
        amount: 50_000_000, // 50 shares currently held
        entry_price_usd: 190.0,
        current_price_usd: 200.0,
        current_value_usd: 10_000.0, // $10,000 existing position
        target_weight_bps: 1500,
        current_weight_bps: 1000, // 10%
        last_rebalanced_at: Some(now_chrono),
        updated_at: now_chrono,
    }];
    let total_portfolio_usd = 100_000u64; // $100,000 total
    let available_cash_usd = 40_000u64; // $40,000 cash (40% cash, > 10% minimum reserve)

    let shariah_record = create_test_shariah_record(now_ts);
    let validation_context = ValidationContext {
        policy: &policy,
        positions: &positions,
        total_portfolio_usd,
        available_cash_usd,
        canonical_asset: Some(&activated_asset),
        shariah_record: Some(&shariah_record),
        shariah_registry: None,
        oracle_price: Some(&pyth_price),
        dex_quote: Some(&dex_quote),
        evaluation_time: now_chrono,
        max_trade_size_usd: None,
        max_oracle_conf_bps: None,
        max_allowed_slippage_bps: None,
        max_allowed_price_impact_bps: None,
        max_oracle_staleness_secs: None,
    };

    let authorizer = DeterministicPolicyAuthorizer::new();
    let auth_outcome = authorizer.authorize(&proposal, &validation_context);
    assert!(
        auth_outcome.is_authorized(),
        "Step 12: Baseline proposal must pass deterministic validation"
    );
    let authorization: ExecutionAuthorization = auth_outcome
        .authorization()
        .expect("Authorization object must exist")
        .clone();
    assert_eq!(authorization.proposal_id, proposal_id);
    assert_eq!(authorization.asset_id, AAPL_ASSET_ID);
    assert_eq!(authorization.authorized_amount, 25_000_000);
    assert_eq!(authorization.dex_minimum_output_amount, 24_875_000);

    // -------------------------------------------------------------------------
    // STEP 13: ExecutionPlan is generated
    // -------------------------------------------------------------------------
    let planner = ExecutionPlanner::new();
    let plan: ExecutionPlan = planner
        .plan_execution(
            &authorization,
            &dex_quote,
            AAPL_FEED,
            conf_bps,
            Some("client-nonce-001"),
            now_ts,
        )
        .await
        .expect("Step 13: Failed to generate deterministic ExecutionPlan");

    assert_eq!(plan.policy_decision_id, authorization.authorization_id);
    assert_eq!(plan.asset_id, AAPL_ASSET_ID);
    assert_eq!(plan.symbol, AAPL_SYMBOL);
    assert_eq!(plan.input_mint, USDC_MINT);
    assert_eq!(plan.output_mint, AAPL_MINT);
    assert_eq!(plan.input_amount, 5_000_000_000);
    assert_eq!(plan.minimum_output_amount, 24_875_000);
    assert_eq!(plan.quote_id, dex_quote.quote_id);
    assert_eq!(plan.oracle_reference.feed_id, AAPL_FEED);

    // Verify canonical serialization is deterministic
    let bytes1 = plan.to_canonical_bytes();
    let bytes2 = plan.to_canonical_bytes();
    assert_eq!(
        bytes1, bytes2,
        "Step 13: Canonical plan serialization must be strictly deterministic"
    );

    // -------------------------------------------------------------------------
    // STEP 14: External signer signs
    // -------------------------------------------------------------------------
    let test_signer = DevTestSigner::new_ephemeral();
    let signer_pubkey = test_signer.pubkey();
    let recent_blockhash = Hash::new_unique();

    let unsigned_tx =
        TransactionBuilder::build_from_plan(&plan, &signer_pubkey, recent_blockhash, &[])
            .expect("Step 14: Failed to construct unsigned transaction");

    let signed_tx = TransactionSignerService::sign(&test_signer, &unsigned_tx)
        .await
        .expect("Step 14: External signer failed to sign transaction");

    assert_eq!(signed_tx.signer_pubkey, signer_pubkey);
    assert_ne!(
        signed_tx.primary_signature,
        solana_sdk::signature::Signature::default(),
        "Step 14: Signature must not be empty/default"
    );
    assert!(
        signed_tx.signer_type == "DevTestSigner",
        "Step 14: Expected DevTestSigner type"
    );

    // -------------------------------------------------------------------------
    // STEP 15: Transaction is submitted
    // -------------------------------------------------------------------------
    let wire_bytes = signed_tx
        .to_bytes()
        .expect("Step 15: Failed to serialize signed transaction wire bytes");
    assert!(!wire_bytes.is_empty());

    // Broadcast simulation produces transaction signature
    let tx_signature = signed_tx.primary_signature.to_string();
    assert!(!tx_signature.is_empty());

    // -------------------------------------------------------------------------
    // STEP 16: Anchor program executes DEX CPI
    // -------------------------------------------------------------------------
    let vault_output_account = Pubkey::new_unique();
    let keeper_pubkey = signer_pubkey;
    let vault_pda = Pubkey::new_unique();
    let input_mint_pubkey = Pubkey::new_unique();
    let output_mint_pubkey = Pubkey::new_unique();
    let (compliance_pda, _) = find_compliance_pda(&output_mint_pubkey, &program_id());

    // Build the Anchor CPI execution instruction
    let anchor_client = AnchorClient::new(Arc::new(
        equity_catalyst_solana::rpc::SolanaRpcClient::new("http://localhost:8899".to_string()),
    ));
    let (execute_ix, execution_pda) = anchor_client
        .build_execute_action_ix(
            &keeper_pubkey,
            &vault_pda,
            1001, // execution seq
            1,    // Spot swap
            &input_mint_pubkey,
            &output_mint_pubkey,
            &compliance_pda,
            plan.input_amount,
            plan.minimum_output_amount,
        )
        .expect("Step 16: Failed to build Anchor execute_action instruction");
    assert_eq!(execute_ix.accounts.len(), 11);
    assert_eq!(execution_pda, execute_ix.accounts[2].pubkey);

    // Pre-execution snapshot
    let before_balance: u64 = 100_000_000; // Pre-CPI balance: 100 AAPL atomic units
    let mut tracker = ExecutionBalanceTracker::new(
        vault_output_account,
        plan.input_amount,
        plan.minimum_output_amount,
        plan.quote_id.clone(),
        plan.policy_decision_id,
    );
    tracker.record_before_balance(before_balance);

    // CPI executes on-chain and delivers 24_950_000 units (> min 24_875_000)
    let actual_delivered: u64 = 24_950_000;
    let after_balance: u64 = before_balance + actual_delivered;
    tracker.record_after_balance(after_balance);

    // -------------------------------------------------------------------------
    // STEP 17: Actual output is measured
    // -------------------------------------------------------------------------
    let measured_output = tracker
        .calculate_actual_output(false)
        .expect("Step 17: Failed to calculate actual output from balance delta");
    assert_eq!(
        measured_output, actual_delivered,
        "Step 17: Measured output must equal real balance delta"
    );

    // Invariants: Never derived from minimum, expected, or input amounts
    assert_ne!(
        measured_output, plan.minimum_output_amount,
        "Step 17: Actual output must not be derived from minimum output"
    );
    assert_ne!(
        measured_output, plan.expected_output_amount,
        "Step 17: Actual output must not be derived from expected output"
    );
    assert_ne!(
        measured_output, plan.input_amount,
        "Step 17: Actual output must not be derived from input amount"
    );

    // -------------------------------------------------------------------------
    // STEP 18: Execution record is stored
    // -------------------------------------------------------------------------
    let record: ExecutionRecord = tracker
        .complete(
            plan.plan_id,
            plan.vault_address.clone(),
            plan.input_mint.clone(),
            plan.output_mint.clone(),
            tx_signature.clone(),
            now_ts,
            false,
        )
        .expect("Step 18: Failed to complete ExecutionRecord");

    // Verify all 7 required recorded fields
    assert_eq!(record.requested_input, plan.input_amount);
    assert_eq!(record.minimum_output, plan.minimum_output_amount);
    assert_eq!(record.actual_output, actual_delivered);
    assert_eq!(record.execution_timestamp, now_ts);
    assert_eq!(record.transaction_signature, tx_signature);
    assert_eq!(record.quote_id, plan.quote_id);
    assert_eq!(record.policy_decision_id, plan.policy_decision_id);
    assert_eq!(record.status, "confirmed");

    // In-memory execution store simulation
    let execution_store: Arc<RwLock<HashMap<Uuid, ExecutionRecord>>> =
        Arc::new(RwLock::new(HashMap::new()));
    execution_store
        .write()
        .await
        .insert(record.execution_id, record.clone());

    let stored_record = execution_store
        .read()
        .await
        .get(&record.execution_id)
        .cloned()
        .expect("Step 18: ExecutionRecord must be retrievable from execution store");
    assert_eq!(stored_record.actual_output, actual_delivered);
    assert_eq!(stored_record.policy_decision_id, plan.policy_decision_id);
}

// =============================================================================
// FAILURE CASE 1: ASSET DEACTIVATED BEFORE EXECUTION
// =============================================================================

#[tokio::test]
async fn test_failure_case_asset_deactivated_before_execution() {
    let now = Utc::now();
    let now_ts = now.timestamp();

    // 1. Setup asset and deactivate it
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let create_req = CreateAssetRequest {
        asset_id: AAPL_ASSET_ID.to_string(),
        symbol: AAPL_SYMBOL.to_string(),
        mint_address: AAPL_MINT.to_string(),
        legal_issuer: "Backed Finance AG".to_string(),
        custodian: "Incore Bank AG".to_string(),
        underlying_asset_identifier: "NASDAQ:AAPL".to_string(),
        is_active: true,
        approval_status: Some(AssetApprovalStatus::ShariahApproved),
        decimals: 8,
    };
    asset_repo.create_asset(&create_req).await.unwrap();
    // Asset is deactivated prior to execution
    let deactivated = asset_repo.deactivate_asset(AAPL_ASSET_ID).await.unwrap();
    assert!(!deactivated.is_active);

    // 2. Policy authorizer rejects proposal
    let policy = create_test_policy(now);
    let positions = vec![];
    let shariah_record = create_test_shariah_record(now_ts);
    let price = MarketPriceUpdate::from_raw(
        AAPL_ASSET_ID,
        AAPL_SYMBOL,
        AAPL_MINT,
        AAPL_FEED,
        20_000_000_000,
        -8,
        20_000_000,
        now_ts - 2,
        30,
        Some(100),
    )
    .unwrap();

    let quoter = MockDexQuoter::new("jupiter-v6");
    quoter.add_pair(USDC_MINT, AAPL_MINT, 0.005, 10);
    let quote = quoter
        .get_executable_quote(&DexQuoteRequest::new(USDC_MINT, AAPL_MINT, 1_000_000_000))
        .await
        .unwrap();

    let proposal = AllocationProposal {
        proposal_id: Uuid::new_v4(),
        vault_address: VAULT_ADDR.to_string(),
        asset_id: AAPL_ASSET_ID.to_string(),
        symbol: AAPL_SYMBOL.to_string(),
        mint_address: AAPL_MINT.to_string(),
        is_buy: true,
        target_amount: 5_000_000,
        proposed_usd_value: 1_000,
        max_slippage_bps: 50,
        proposed_at: now,
        ai_confidence: Some(0.99),
        ai_rationale: None,
        ai_approved: true, // Attempted AI bypass
    };

    let context = ValidationContext {
        policy: &policy,
        positions: &positions,
        total_portfolio_usd: 100_000,
        available_cash_usd: 50_000,
        canonical_asset: Some(&deactivated),
        shariah_record: Some(&shariah_record),
        shariah_registry: None,
        oracle_price: Some(&price),
        dex_quote: Some(&quote),
        evaluation_time: now,
        max_trade_size_usd: None,
        max_oracle_conf_bps: None,
        max_allowed_slippage_bps: None,
        max_allowed_price_impact_bps: None,
        max_oracle_staleness_secs: None,
    };

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&proposal, &context);
    assert!(
        !outcome.is_authorized(),
        "Deactivated asset MUST be rejected"
    );
    let reasons = outcome
        .rejection_reasons()
        .expect("Rejection reasons must exist");
    match &reasons[0] {
        PolicyRejectionReason::AssetInactive { asset_id, symbol } => {
            assert_eq!(asset_id, AAPL_ASSET_ID);
            assert_eq!(symbol, AAPL_SYMBOL);
        }
        other => panic!("Expected AssetInactive rejection, got {:?}", other),
    }

    // 3. Safety guard fail-closed check
    let guard = TradingPipelineSafetyGuard::new();
    let price_ok = Ok(price);
    let dex_ok = Ok(());
    let sub_ok = Ok("sig".to_string());
    let db_ok = Ok(());
    let exec_context = PipelineExecutionContext {
        symbol: AAPL_SYMBOL,
        expected_feed_id: AAPL_FEED,
        price_update: &price_ok,
        is_stream_connected: true,
        is_asset_active: false, // Deactivated
        asset_status: "Deactivated",
        shariah_status: ShariahStatus::Approved,
        shariah_reviewed_at: now_ts - 100,
        shariah_expires_at: now_ts + 10000,
        quote_id: "q-1",
        quote_expires_at: now_ts + 60,
        dex_result: &dex_ok,
        is_signer_available: true,
        signer_type: "DevTestSigner",
        submission_result: &sub_ok,
        is_confirmed: true,
        confirmation_timeout_secs: 30,
        execution_id: Uuid::new_v4(),
        is_duplicate: false,
        existing_status: None,
        db_result: &db_ok,
        is_ws_connected: true,
        now: now_ts,
    };
    let eval = guard.evaluate_pipeline_safety(&exec_context);
    assert_eq!(
        eval.unwrap_err(),
        PipelineFailure::AssetDeactivated {
            symbol: AAPL_SYMBOL.to_string(),
            status: "Deactivated".to_string()
        }
    );
}

// =============================================================================
// FAILURE CASE 2: STALE ORACLE
// =============================================================================

#[tokio::test]
async fn test_failure_case_stale_oracle() {
    let now = Utc::now();
    let now_ts = now.timestamp();

    // Oracle price published 45s ago (limit is 30s)
    let stale_publish_time = now_ts - 45;
    let stale_price = MarketPriceUpdate::from_raw(
        AAPL_ASSET_ID,
        AAPL_SYMBOL,
        AAPL_MINT,
        AAPL_FEED,
        20_000_000_000,
        -8,
        20_000_000,
        stale_publish_time,
        30,
        Some(100),
    )
    .unwrap();

    assert!(stale_price.is_stale);
    assert_eq!(stale_price.freshness_status(30), PriceFreshness::Stale);

    // Policy authorizer evaluation
    let policy = create_test_policy(now);
    let canonical_asset = CanonicalAssetModel {
        asset_id: AAPL_ASSET_ID.to_string(),
        symbol: AAPL_SYMBOL.to_string(),
        mint_address: AAPL_MINT.to_string(),
        legal_issuer: "Backed Finance AG".to_string(),
        custodian: "Incore Bank AG".to_string(),
        underlying_asset_identifier: "US0378331005".to_string(),
        is_active: true,
        approval_status: AssetApprovalStatus::ShariahApproved,
        decimals: 8,
        created_at: now,
        updated_at: now,
    };
    let shariah_record = create_test_shariah_record(now_ts);

    let quoter = MockDexQuoter::new("jupiter-v6");
    quoter.add_pair(USDC_MINT, AAPL_MINT, 0.005, 10);
    let quote = quoter
        .get_executable_quote(&DexQuoteRequest::new(USDC_MINT, AAPL_MINT, 1_000_000_000))
        .await
        .unwrap();

    let proposal = AllocationProposal {
        proposal_id: Uuid::new_v4(),
        vault_address: VAULT_ADDR.to_string(),
        asset_id: AAPL_ASSET_ID.to_string(),
        symbol: AAPL_SYMBOL.to_string(),
        mint_address: AAPL_MINT.to_string(),
        is_buy: true,
        target_amount: 5_000_000,
        proposed_usd_value: 1_000,
        max_slippage_bps: 50,
        proposed_at: now,
        ai_confidence: Some(0.95),
        ai_rationale: None,
        ai_approved: false,
    };

    let context = ValidationContext {
        policy: &policy,
        positions: &[],
        total_portfolio_usd: 100_000,
        available_cash_usd: 50_000,
        canonical_asset: Some(&canonical_asset),
        shariah_record: Some(&shariah_record),
        shariah_registry: None,
        oracle_price: Some(&stale_price),
        dex_quote: Some(&quote),
        evaluation_time: now,
        max_trade_size_usd: None,
        max_oracle_conf_bps: None,
        max_allowed_slippage_bps: None,
        max_allowed_price_impact_bps: None,
        max_oracle_staleness_secs: Some(30),
    };

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&proposal, &context);
    assert!(!outcome.is_authorized(), "Stale oracle MUST be rejected");
    let reasons = outcome
        .rejection_reasons()
        .expect("Rejection reasons must exist");
    match &reasons[0] {
        PolicyRejectionReason::OraclePriceStale {
            symbol,
            publish_time,
            max_staleness_secs,
            age_secs,
        } => {
            assert_eq!(symbol, AAPL_SYMBOL);
            assert_eq!(*publish_time, stale_publish_time);
            assert_eq!(*max_staleness_secs, 30);
            assert_eq!(*age_secs, 45);
        }
        other => panic!("Expected OraclePriceStale rejection, got {:?}", other),
    }

    // Pipeline safety guard evaluation
    let guard = TradingPipelineSafetyGuard::new();
    let price_ok = Ok(stale_price);
    let dex_ok = Ok(());
    let sub_ok = Ok("sig".to_string());
    let db_ok = Ok(());
    let exec_context = PipelineExecutionContext {
        symbol: AAPL_SYMBOL,
        expected_feed_id: AAPL_FEED,
        price_update: &price_ok,
        is_stream_connected: true,
        is_asset_active: true,
        asset_status: "Active",
        shariah_status: ShariahStatus::Approved,
        shariah_reviewed_at: now_ts - 100,
        shariah_expires_at: now_ts + 10000,
        quote_id: "q-1",
        quote_expires_at: now_ts + 60,
        dex_result: &dex_ok,
        is_signer_available: true,
        signer_type: "DevTestSigner",
        submission_result: &sub_ok,
        is_confirmed: true,
        confirmation_timeout_secs: 30,
        execution_id: Uuid::new_v4(),
        is_duplicate: false,
        existing_status: None,
        db_result: &db_ok,
        is_ws_connected: true,
        now: now_ts,
    };
    let eval = guard.evaluate_pipeline_safety(&exec_context);
    assert_eq!(
        eval.unwrap_err(),
        PipelineFailure::PythDataStale {
            symbol: AAPL_SYMBOL.to_string(),
            publish_time: stale_publish_time,
            age_secs: 45,
            max_staleness_secs: 30
        }
    );
}

// =============================================================================
// FAILURE CASE 3: WRONG FEED
// =============================================================================

#[tokio::test]
async fn test_failure_case_wrong_feed() {
    let now = Utc::now();
    let now_ts = now.timestamp();

    // Price update arrives with wrong feed ID
    let wrong_feed_price = MarketPriceUpdate::from_raw(
        AAPL_ASSET_ID,
        AAPL_SYMBOL,
        AAPL_MINT,
        WRONG_FEED,
        20_000_000_000,
        -8,
        20_000_000,
        now_ts - 2,
        30,
        Some(100),
    )
    .unwrap();

    let guard = TradingPipelineSafetyGuard::new();
    let price_ok = Ok(wrong_feed_price);
    let dex_ok = Ok(());
    let sub_ok = Ok("sig".to_string());
    let db_ok = Ok(());
    let exec_context = PipelineExecutionContext {
        symbol: AAPL_SYMBOL,
        expected_feed_id: AAPL_FEED, // Canonical feed
        price_update: &price_ok,
        is_stream_connected: true,
        is_asset_active: true,
        asset_status: "Active",
        shariah_status: ShariahStatus::Approved,
        shariah_reviewed_at: now_ts - 100,
        shariah_expires_at: now_ts + 10000,
        quote_id: "q-1",
        quote_expires_at: now_ts + 60,
        dex_result: &dex_ok,
        is_signer_available: true,
        signer_type: "DevTestSigner",
        submission_result: &sub_ok,
        is_confirmed: true,
        confirmation_timeout_secs: 30,
        execution_id: Uuid::new_v4(),
        is_duplicate: false,
        existing_status: None,
        db_result: &db_ok,
        is_ws_connected: true,
        now: now_ts,
    };
    let eval = guard.evaluate_pipeline_safety(&exec_context);
    assert_eq!(
        eval.unwrap_err(),
        PipelineFailure::WrongFeed {
            symbol: AAPL_SYMBOL.to_string(),
            expected_feed_id: equity_catalyst_pyth::normalize_feed_id(AAPL_FEED),
            received_feed_id: equity_catalyst_pyth::normalize_feed_id(WRONG_FEED)
        }
    );
}

// =============================================================================
// FAILURE CASE 4: EXPIRED QUOTE
// =============================================================================

#[tokio::test]
async fn test_failure_case_expired_quote() {
    let now = Utc::now();
    let now_ts = now.timestamp();

    // Expired quote: expired 10 seconds ago
    let expired_quote = DexQuote {
        quote_id: "quote-expired-001".to_string(),
        provider_id: "jupiter-v6".to_string(),
        input_mint: USDC_MINT.to_string(),
        output_mint: AAPL_MINT.to_string(),
        input_amount: 1_000_000_000,
        expected_output_amount: 5_000_000,
        minimum_output_amount: 4_975_000,
        price_impact_bps: 10,
        price_impact_pct: "0.10%".to_string(),
        effective_rate: 0.005,
        quote_timestamp: now_ts - 70,
        expires_at: now_ts - 10, // Expired
        ttl_seconds: 60,
        route_info: DexRouteInfo {
            steps: vec![],
            num_hops: 1,
            primary_dex: "Orca Whirlpool".to_string(),
        },
        raw_payload: None,
    };
    assert!(expired_quote.is_expired(now_ts));

    // Execution planner rejects expired quote
    let auth = ExecutionAuthorization {
        authorization_id: Uuid::new_v4(),
        proposal_id: Uuid::new_v4(),
        vault_address: VAULT_ADDR.to_string(),
        asset_id: AAPL_ASSET_ID.to_string(),
        symbol: AAPL_SYMBOL.to_string(),
        mint_address: AAPL_MINT.to_string(),
        is_buy: true,
        authorized_amount: 5_000_000,
        authorized_usd_value: 1_000,
        max_slippage_bps: 50,
        oracle_reference_price_scaled: 200_000_000,
        oracle_publish_time: now_ts - 2,
        dex_expected_output_amount: 5_000_000,
        dex_minimum_output_amount: 4_975_000,
        dex_price_impact_bps: 10,
        authorized_at: now,
        valid_until: now + chrono::Duration::seconds(60),
        audit_digest: "audit-001".to_string(),
    };

    let planner = ExecutionPlanner::new();
    let plan_res = planner
        .plan_execution(&auth, &expired_quote, AAPL_FEED, 10, None, now_ts)
        .await;

    assert_eq!(
        plan_res.unwrap_err(),
        PlannerError::QuoteExpired {
            quote_id: "quote-expired-001".to_string(),
            expires_at: now_ts - 10,
            current_time: now_ts,
        }
    );

    // Safety guard rejects expired quote
    let guard = TradingPipelineSafetyGuard::new();
    let price_ok = Ok(MarketPriceUpdate::from_raw(
        AAPL_ASSET_ID,
        AAPL_SYMBOL,
        AAPL_MINT,
        AAPL_FEED,
        20_000_000_000,
        -8,
        20_000_000,
        now_ts - 2,
        30,
        Some(100),
    )
    .unwrap());
    let dex_ok = Ok(());
    let sub_ok = Ok("sig".to_string());
    let db_ok = Ok(());
    let exec_context = PipelineExecutionContext {
        symbol: AAPL_SYMBOL,
        expected_feed_id: AAPL_FEED,
        price_update: &price_ok,
        is_stream_connected: true,
        is_asset_active: true,
        asset_status: "Active",
        shariah_status: ShariahStatus::Approved,
        shariah_reviewed_at: now_ts - 100,
        shariah_expires_at: now_ts + 10000,
        quote_id: &expired_quote.quote_id,
        quote_expires_at: expired_quote.expires_at, // Expired
        dex_result: &dex_ok,
        is_signer_available: true,
        signer_type: "DevTestSigner",
        submission_result: &sub_ok,
        is_confirmed: true,
        confirmation_timeout_secs: 30,
        execution_id: Uuid::new_v4(),
        is_duplicate: false,
        existing_status: None,
        db_result: &db_ok,
        is_ws_connected: true,
        now: now_ts,
    };
    let eval = guard.evaluate_pipeline_safety(&exec_context);
    assert_eq!(
        eval.unwrap_err(),
        PipelineFailure::DexQuoteExpired {
            quote_id: "quote-expired-001".to_string(),
            expires_at: now_ts - 10,
            now: now_ts,
        }
    );
}

// =============================================================================
// FAILURE CASE 5: EXCESSIVE SLIPPAGE
// =============================================================================

#[test]
fn test_failure_case_excessive_slippage() {
    let mut tracker = ExecutionBalanceTracker::new(
        Pubkey::new_unique(),
        1_000_000_000, // input 1000 USDC
        5_000_000,     // min output 5.0 AAPL
        "quote-slip-001".to_string(),
        Uuid::new_v4(),
    );

    // Pre-CPI balance: 10_000_000
    tracker.record_before_balance(10_000_000);

    // CPI delivers only 4_800_000 (< minimum 5_000_000)
    // Post-CPI balance: 14_800_000
    tracker.record_after_balance(14_800_000);

    let err = tracker
        .calculate_actual_output(false)
        .expect_err("Must reject when actual output breaches slippage limit");

    assert_eq!(
        err,
        ExecutionRecorderError::SlippageExceeded {
            actual: 4_800_000,
            minimum: 5_000_000
        }
    );

    // Ensure completion also fails closed
    let complete_err = tracker
        .complete(
            Uuid::new_v4(),
            VAULT_ADDR.to_string(),
            USDC_MINT.to_string(),
            AAPL_MINT.to_string(),
            "sig".to_string(),
            1720000000,
            false,
        )
        .expect_err("Tracker completion must fail closed on excessive slippage");
    assert_eq!(
        complete_err,
        ExecutionRecorderError::SlippageExceeded {
            actual: 4_800_000,
            minimum: 5_000_000
        }
    );
}

// =============================================================================
// FAILURE CASE 6: SIGNER UNAVAILABLE
// =============================================================================

#[tokio::test]
async fn test_failure_case_signer_unavailable() {
    let unavailable_signer = UnavailableSigner::new();
    assert!(!unavailable_signer.is_available());

    let plan = ExecutionPlan {
        plan_id: Uuid::new_v4(),
        policy_decision_id: Uuid::new_v4(),
        idempotency_key: "idemp:test:offline".to_string(),
        vault_address: VAULT_ADDR.to_string(),
        asset_id: AAPL_ASSET_ID.to_string(),
        symbol: AAPL_SYMBOL.to_string(),
        is_buy: true,
        input_mint: USDC_MINT.to_string(),
        output_mint: AAPL_MINT.to_string(),
        input_amount: 1_000_000,
        expected_output_amount: 5_000,
        minimum_output_amount: 4_950,
        slippage_bps: 50,
        quote_id: "q-1".to_string(),
        quote_timestamp: 1720000000,
        oracle_reference: equity_catalyst_api::engines::execution_planner::OracleReferenceInfo {
            price_scaled: 200_000_000,
            publish_time: 1720000000,
            conf_bps: 10,
            feed_id: AAPL_FEED.to_string(),
        },
        expires_at: 1720000060,
        created_at: 1720000000,
    };

    let unsigned = TransactionBuilder::build_from_plan(
        &plan,
        &unavailable_signer.pubkey(),
        Hash::new_unique(),
        &[],
    )
    .unwrap();

    let err = TransactionSignerService::sign(&unavailable_signer, &unsigned)
        .await
        .expect_err("SignerService must fail closed when signer is unavailable");

    match err {
        SignerError::SignerUnavailable {
            signer_type,
            reason,
        } => {
            assert_eq!(signer_type, "UnavailableSigner");
            assert!(reason.contains("offline or disconnected"));
        }
        other => panic!("Expected SignerUnavailable error, got {:?}", other),
    }
}

// =============================================================================
// FAILURE CASE 7: DUPLICATE EXECUTION
// =============================================================================

#[tokio::test]
async fn test_failure_case_duplicate_execution() {
    let now = Utc::now();
    let now_ts = now.timestamp();

    let auth = ExecutionAuthorization {
        authorization_id: Uuid::new_v4(),
        proposal_id: Uuid::new_v4(),
        vault_address: VAULT_ADDR.to_string(),
        asset_id: AAPL_ASSET_ID.to_string(),
        symbol: AAPL_SYMBOL.to_string(),
        mint_address: AAPL_MINT.to_string(),
        is_buy: true,
        authorized_amount: 5_000_000,
        authorized_usd_value: 1_000,
        max_slippage_bps: 50,
        oracle_reference_price_scaled: 200_000_000,
        oracle_publish_time: now_ts - 2,
        dex_expected_output_amount: 5_000_000,
        dex_minimum_output_amount: 4_975_000,
        dex_price_impact_bps: 10,
        authorized_at: now,
        valid_until: now + chrono::Duration::seconds(60),
        audit_digest: "audit-dup-001".to_string(),
    };

    let quoter = MockDexQuoter::new("jupiter-v6");
    quoter.add_pair(USDC_MINT, AAPL_MINT, 0.005, 10);
    let quote = quoter
        .get_executable_quote(&DexQuoteRequest::new(USDC_MINT, AAPL_MINT, 1_000_000_000))
        .await
        .unwrap();

    let tracker = Arc::new(IdempotencyTracker::new());
    let planner = ExecutionPlanner::with_tracker(tracker);

    // First planning call succeeds
    let plan1 = planner
        .plan_execution(
            &auth,
            &quote,
            AAPL_FEED,
            10,
            Some("duplicate-test-nonce"),
            now_ts,
        )
        .await
        .expect("First plan execution must succeed");
    assert_eq!(plan1.policy_decision_id, auth.authorization_id);

    // Second planning call with identical decision fails closed as duplicate
    let plan2_err = planner
        .plan_execution(
            &auth,
            &quote,
            AAPL_FEED,
            10,
            Some("duplicate-test-nonce-diff"),
            now_ts,
        )
        .await
        .expect_err("Second plan execution with same decision MUST fail closed");

    match plan2_err {
        PlannerError::DuplicateExecutionPlan { decision_id, .. } => {
            assert_eq!(decision_id, auth.authorization_id);
        }
        other => panic!("Expected DuplicateExecutionPlan error, got {:?}", other),
    }

    // Pipeline safety guard rejects duplicate execution request
    let guard = TradingPipelineSafetyGuard::new();
    let price_ok = Ok(MarketPriceUpdate::from_raw(
        AAPL_ASSET_ID,
        AAPL_SYMBOL,
        AAPL_MINT,
        AAPL_FEED,
        20_000_000_000,
        -8,
        20_000_000,
        now_ts - 2,
        30,
        Some(100),
    )
    .unwrap());
    let dex_ok = Ok(());
    let sub_ok = Ok("sig".to_string());
    let db_ok = Ok(());
    let exec_context = PipelineExecutionContext {
        symbol: AAPL_SYMBOL,
        expected_feed_id: AAPL_FEED,
        price_update: &price_ok,
        is_stream_connected: true,
        is_asset_active: true,
        asset_status: "Active",
        shariah_status: ShariahStatus::Approved,
        shariah_reviewed_at: now_ts - 100,
        shariah_expires_at: now_ts + 10000,
        quote_id: "q-1",
        quote_expires_at: now_ts + 60,
        dex_result: &dex_ok,
        is_signer_available: true,
        signer_type: "DevTestSigner",
        submission_result: &sub_ok,
        is_confirmed: true,
        confirmation_timeout_secs: 30,
        execution_id: plan1.plan_id,
        is_duplicate: true, // Marked as duplicate
        existing_status: Some("confirmed"),
        db_result: &db_ok,
        is_ws_connected: true,
        now: now_ts,
    };
    let eval = guard.evaluate_pipeline_safety(&exec_context);
    assert_eq!(
        eval.unwrap_err(),
        PipelineFailure::DuplicateExecutionRequest {
            execution_id: plan1.plan_id,
            existing_status: "confirmed".to_string()
        }
    );
}

// =============================================================================
// FAILURE CASE 8: FAILED TRANSACTION
// =============================================================================

#[test]
fn test_failure_case_failed_transaction_negative_balance_delta() {
    let mut tracker = ExecutionBalanceTracker::new(
        Pubkey::new_unique(),
        1_000_000,
        900_000,
        "q-fail".to_string(),
        Uuid::new_v4(),
    );

    // Pre-balance: 10_000_000
    tracker.record_before_balance(10_000_000);

    // Post-balance dropped to 9_000_000 (negative delta!)
    tracker.record_after_balance(9_000_000);

    let err = tracker
        .calculate_actual_output(false)
        .expect_err("Negative balance delta MUST fail closed");

    assert_eq!(
        err,
        ExecutionRecorderError::NegativeBalanceDelta {
            before: 10_000_000,
            after: 9_000_000
        }
    );
}

#[tokio::test]
async fn test_failure_case_failed_transaction_submission_error() {
    let now = Utc::now();
    let now_ts = now.timestamp();

    // Submission fails with network / RPC error
    let guard = TradingPipelineSafetyGuard::new();
    let price_ok = Ok(MarketPriceUpdate::from_raw(
        AAPL_ASSET_ID,
        AAPL_SYMBOL,
        AAPL_MINT,
        AAPL_FEED,
        20_000_000_000,
        -8,
        20_000_000,
        now_ts - 2,
        30,
        Some(100),
    )
    .unwrap());
    let dex_ok = Ok(());
    let sub_err = Err("Solana RPC 503 node out of sync: blockhash expired".to_string());
    let db_ok = Ok(());

    let exec_context = PipelineExecutionContext {
        symbol: AAPL_SYMBOL,
        expected_feed_id: AAPL_FEED,
        price_update: &price_ok,
        is_stream_connected: true,
        is_asset_active: true,
        asset_status: "Active",
        shariah_status: ShariahStatus::Approved,
        shariah_reviewed_at: now_ts - 100,
        shariah_expires_at: now_ts + 10000,
        quote_id: "q-1",
        quote_expires_at: now_ts + 60,
        dex_result: &dex_ok,
        is_signer_available: true,
        signer_type: "DevTestSigner",
        submission_result: &sub_err, // Failed submission
        is_confirmed: false,
        confirmation_timeout_secs: 30,
        execution_id: Uuid::new_v4(),
        is_duplicate: false,
        existing_status: None,
        db_result: &db_ok,
        is_ws_connected: true,
        now: now_ts,
    };

    let eval = guard.evaluate_pipeline_safety(&exec_context);
    assert_eq!(
        eval.unwrap_err(),
        PipelineFailure::TransactionSubmissionFailure {
            details: "Solana RPC 503 node out of sync: blockhash expired".to_string()
        }
    );
}
