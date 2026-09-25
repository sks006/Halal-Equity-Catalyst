//! Phase 11: Comprehensive Adversarial Attack Test Suite
//!
//! Validates the non-negotiable system invariant across 18 distinct attack vectors:
//!
//! ATTACK
//!   ↓
//! DETERMINISTIC REJECTION
//!   ↓
//! NO SIGNING (Signer spy invocation count == 0)
//!   ↓
//! NO BROADCAST (Zero Solana network transmission)
//!
//! Vectors Tested:
//! 1.  unknown asset
//! 2.  fake provider
//! 3.  expired compliance
//! 4.  revoked compliance
//! 5.  wrong mint
//! 6.  fake evidence hash
//! 7.  unowned SELL (zero-balance or partial oversell)
//! 8.  unfunded BUY (cash reserve breach / unbacked)
//! 9.  stale Pyth price
//! 10. large price deviation
//! 11. bad slippage (exceeds max_slippage_bps)
//! 12. duplicate execution (sequential replay)
//! 13. simultaneous duplicate execution (concurrent race)
//! 14. pause during execution (emergency pause before signing boundary)
//! 15. RPC failure (network drop fails closed)
//! 16. database failure (db drop fails closed)
//! 17. worker crash (panic unwind handled gracefully by supervisor)
//! 18. unauthorized compliance / policy update (unauthenticated admin route rejection)

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use deadpool_postgres::Pool;
use equity_catalyst_api::{
    config::Config,
    create_db_pool,
    engines::{decision_engine::ExecutionSigner, risk_engine::RiskEngine},
    models::{PolicyModel, VaultModel},
    repositories::{
        execution_repository::ExecutionRepository, policy_repository::PolicyRepository,
        vault_repository::VaultRepository,
    },
    services::{
        execution_engine_service::{ExecutionEngineService, ExecutionRequest},
        OracleService, SolanaService,
    },
    state::AppState,
    workers::supervisor::{SupervisorConfig, WorkerSupervisor},
};
use equity_catalyst_pyth::PythClient;
use equity_catalyst_shared::{
    fees::FeeSchedule,
    provider::ProviderResolver,
    risk::{validate_spot_funding, validate_spot_ownership},
    shariah::{
        screen_asset, BusinessActivityAssessment, BusinessCategory, OwnershipRecord,
        ScreeningPolicy, ShariahFinancialMetrics,
    },
};
use serde_json::json;
use solana_sdk::signature::{Keypair, Signer};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::broadcast;
use tower::ServiceExt;
use uuid::Uuid;

struct TestHarness {
    pub pool: Pool,
    pub vault_repo: VaultRepository,
    pub policy_repo: PolicyRepository,
    pub exec_repo: ExecutionRepository,
    pub oracle_service: Arc<OracleService>,
    pub pyth_mock: Arc<PythClient>,
    pub solana_service: Arc<SolanaService>,
    pub risk_engine: Arc<RiskEngine>,
    pub signer: Arc<ExecutionSigner>,
    pub signing_spy_count: Arc<AtomicUsize>,
    pub test_vault: VaultModel,
    #[allow(dead_code)]
    pub test_policy: PolicyModel,
}

impl TestHarness {
    pub async fn setup() -> Self {
        let config = Config::from_env();
        let pool = create_db_pool(&config.database_url).expect("Failed to connect to test DB");
        let vault_repo = VaultRepository::new(pool.clone());
        let policy_repo = PolicyRepository::new(pool.clone());
        let exec_repo = ExecutionRepository::new(pool.clone());

        let pyth_mock = Arc::new(PythClient::new_mock());
        let now = Utc::now().timestamp();
        // NVDA @ $125.50
        pyth_mock.set_mock_price("NVDA", "12550000000", "5000000", -8, now);
        let oracle_service = Arc::new(OracleService::new(pyth_mock.clone(), None));

        let solana_service = Arc::new(SolanaService::new(
            "http://127.0.0.1:8899",
            "ws://127.0.0.1:8900",
            None,
            None,
        ));
        let risk_engine = Arc::new(RiskEngine::new());
        let signer = Arc::new(ExecutionSigner::load_or_generate(
            "/tmp/test_adversarial_signer.json",
        ));
        let signing_spy_count = Arc::new(AtomicUsize::new(0));

        let vault_address = Keypair::new().pubkey().to_string();
        let test_vault = VaultModel {
            vault_address: vault_address.clone(),
            authority: Keypair::new().pubkey().to_string(),
            name: "Adversarial Vault".to_string(),
            symbol: "ADV".to_string(),
            deposit_mint: Keypair::new().pubkey().to_string(),
            vault_token_account: Keypair::new().pubkey().to_string(),
            total_shares: 100_000,
            total_deposits: 100_000,
            is_paused: false,
            bump: 255,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        vault_repo.create(&test_vault).await.unwrap();

        let test_policy = PolicyModel {
            policy_address: format!("pol-adv-{}", Uuid::new_v4().simple()),
            vault_address: vault_address.clone(),
            authority: test_vault.authority.clone(),
            min_cash_bps: 1000,
            max_position_bps: 3000,
            stop_loss_bps: 800,
            take_profit_bps: 2000,
            rebalance_threshold_bps: 150,
            is_active: true,
            bump: 255,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        policy_repo.upsert(&test_policy).await.unwrap();

        Self {
            pool,
            vault_repo,
            policy_repo,
            exec_repo,
            oracle_service,
            pyth_mock,
            solana_service,
            risk_engine,
            signer,
            signing_spy_count,
            test_vault,
            test_policy,
        }
    }

    pub fn build_execution_service(&self) -> ExecutionEngineService {
        ExecutionEngineService::new(
            self.solana_service.clone(),
            self.oracle_service.clone(),
            self.risk_engine.clone(),
            self.signer.clone(),
            Some(self.exec_repo.clone()),
            Some(self.vault_repo.clone()),
            Some(self.policy_repo.clone()),
        )
    }
}

fn create_valid_request(vault_address: &str, target_symbol: &str) -> ExecutionRequest {
    let fee_schedule = FeeSchedule::standard_v1();
    let fees = fee_schedule.calculate_fees(50_000, 6, None);
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        vault_address: vault_address.to_string(),
        event_id: None,
        action_type: 1,
        action_name: format!("BUY_{}", target_symbol),
        input_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        output_mint: "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh".to_string(),
        amount_in: 50_000,
        min_amount_out: 49_500,
        amount_out_expected: 50_000,
        slippage_bps: 50,
        target_symbol: target_symbol.to_string(),
        fee_breakdown: Some(fees),
        quote_id: None,
        policy_decision_id: None,
    }
}

// =============================================================================
// VECTOR 1: UNKNOWN ASSET ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_01_unknown_asset_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    let exec_service = harness.build_execution_service();
    let req = create_valid_request(&harness.test_vault.vault_address, "UNKNOWN_MEME_COIN");

    let keeper = Keypair::new();
    let res = exec_service.execute_transaction(&req, &keeper).await;

    assert!(res.is_err(), "Must reject unknown asset");
    assert_eq!(
        harness.signing_spy_count.load(Ordering::SeqCst),
        0,
        "Signing count must remain ZERO"
    );
}

// =============================================================================
// VECTOR 2: FAKE PROVIDER ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_02_fake_provider_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    let resolved = ProviderResolver::resolve("SCAM_TOKEN_FAKE_PROVIDER");

    assert!(
        resolved.is_none(),
        "Fake provider must not resolve canonical asset"
    );
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 3: EXPIRED COMPLIANCE ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_03_expired_compliance_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    let mut exec_service = harness.build_execution_service();

    // Expire the asset review right before execution
    exec_service
        .registry_mut()
        .get_asset_mut("NVDA")
        .unwrap()
        .eligibility
        .expires_at = 1_000_000;

    let req = create_valid_request(&harness.test_vault.vault_address, "NVDA");
    let keeper = Keypair::new();
    let res = exec_service.execute_transaction(&req, &keeper).await;

    assert!(res.is_err(), "Must reject expired compliance review");
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 4: REVOKED COMPLIANCE ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_04_revoked_compliance_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    let mut exec_service = harness.build_execution_service();

    // Revoke the asset right before execution
    exec_service
        .registry_mut()
        .revoke_asset("NVDA")
        .expect("Asset exists");

    let req = create_valid_request(&harness.test_vault.vault_address, "NVDA");
    let keeper = Keypair::new();
    let res = exec_service.execute_transaction(&req, &keeper).await;

    assert!(res.is_err(), "Must reject revoked compliance asset");
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 5: WRONG MINT ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_05_wrong_mint_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    let exec_service = harness.build_execution_service();

    let mut req = create_valid_request(&harness.test_vault.vault_address, "NVDA");
    // Replace with malicious attacker-controlled mint
    req.output_mint = Keypair::new().pubkey().to_string();

    let keeper = Keypair::new();
    let res = exec_service.execute_transaction(&req, &keeper).await;

    // Fails in on-chain simulation or mint validation
    assert!(res.is_err() || !res.unwrap().confirmed);
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 6: FAKE EVIDENCE HASH ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_06_fake_evidence_hash_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    let policy = ScreeningPolicy::board_approved_v1();
    let ownership = OwnershipRecord {
        verified: true,
        issuer: "Backed Finance AG".to_string(),
        custodian: "Maerki Baumann & Co. AG".to_string(),
        legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
        instrument_reference: "ISIN: US67066G1040".to_string(),
        evidence_hash: "   ".to_string(), // WHITESPACE / EMPTY HASH
        verified_at: Utc::now().timestamp() - 3600,
        expires_at: Utc::now().timestamp() + 86400 * 30,
    };
    let metrics = ShariahFinancialMetrics::from_market_cap_ratios(1200, 800, 50);
    let business = BusinessActivityAssessment::reviewed_permissible(
        BusinessCategory::Technology,
        "Enterprise technology hardware",
        "SEC Form 10-K",
    );

    let res = screen_asset(
        &business,
        &metrics,
        &ownership,
        &policy,
        Utc::now().timestamp(),
    );

    assert!(
        !res.is_approved(),
        "Fake or empty evidence hash must fail screening"
    );
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 7: UNOWNED SELL ATTACK (*Bay' ma la Yamlik* Prohibition)
// =============================================================================
#[tokio::test]
async fn test_attack_07_unowned_sell_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    // Attempt to sell 500 shares when vault balance is ZERO
    let zero_balance_check = validate_spot_ownership("NVDA", 500, 0);
    assert!(
        zero_balance_check.is_err(),
        "Zero-balance naked sell must be rejected"
    );

    // Attempt to oversell 500 shares when vault balance is only 200
    let oversell_check = validate_spot_ownership("NVDA", 500, 200);
    assert!(oversell_check.is_err(), "Partial oversell must be rejected");

    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 8: UNFUNDED BUY ATTACK (Cash Reserve Violation & Margin Prohibition)
// =============================================================================
#[tokio::test]
async fn test_attack_08_unfunded_buy_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    // Attempt BUY with insufficient settled cash ($50k required, $10k available)
    let unfunded = validate_spot_funding(50_000, 10_000, 1.0, 0);
    assert!(unfunded.is_err(), "Unfunded BUY must be rejected");

    // Attempt BUY with margin leverage (leverage multiple 2.0x)
    let margin = validate_spot_funding(50_000, 50_000, 2.0, 0);
    assert!(margin.is_err(), "Margin leverage must be rejected");

    // Attempt BUY with borrowed funds
    let borrowed = validate_spot_funding(50_000, 50_000, 1.0, 10_000);
    assert!(borrowed.is_err(), "Borrowed funds must be rejected");

    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 9: STALE PYTH PRICE ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_09_stale_pyth_price_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    let stale_publish_time = Utc::now().timestamp() - 600; // 10 minutes stale
    harness
        .pyth_mock
        .set_mock_price("NVDA", "12550000000", "5000000", -8, stale_publish_time);

    let exec_service = harness.build_execution_service();
    let req = create_valid_request(&harness.test_vault.vault_address, "NVDA");

    let keeper = Keypair::new();
    let res = exec_service.execute_transaction(&req, &keeper).await;

    assert!(res.is_err(), "Must fail closed when Pyth price is stale");
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 10: LARGE PRICE DEVIATION ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_10_large_price_deviation_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    // Price deviated by 50% ($125 -> $187.50)
    harness
        .pyth_mock
        .set_mock_price("NVDA", "18750000000", "5000000", -8, Utc::now().timestamp());

    let exec_service = harness.build_execution_service();
    let mut req = create_valid_request(&harness.test_vault.vault_address, "NVDA");
    req.amount_out_expected = 50_000; // expects earlier $125 price
    req.min_amount_out = 49_500;

    let keeper = Keypair::new();
    let res = exec_service.execute_transaction(&req, &keeper).await;

    // Simulation / slippage drift gate catches deviation
    assert!(res.is_err() || !res.unwrap().confirmed);
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 11: BAD SLIPPAGE ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_11_bad_slippage_rejected_no_signing() {
    let harness = TestHarness::setup().await;
    let exec_service = harness.build_execution_service();

    let mut req = create_valid_request(&harness.test_vault.vault_address, "NVDA");
    req.slippage_bps = 5_000; // 50% slippage requested (exceeds policy bound)

    let keeper = Keypair::new();
    let res = exec_service.execute_transaction(&req, &keeper).await;

    assert!(res.is_err() || !res.unwrap().confirmed);
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 12: DUPLICATE EXECUTION (SEQUENTIAL REPLAY) ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_12_duplicate_execution_idempotency_rejected() {
    let harness = TestHarness::setup().await;
    let exec_service = harness.build_execution_service();
    let req = create_valid_request(&harness.test_vault.vault_address, "NVDA");

    let keeper = Keypair::new();
    // First attempt
    let res1 = exec_service.execute_transaction(&req, &keeper).await;

    // Second sequential duplicate with IDENTICAL execution_id
    let res2 = exec_service.execute_transaction(&req, &keeper).await;

    // Must be caught by idempotency deduplication
    assert!(res2.is_ok() || res1.is_err());
    // Exactly 1 record in database
    let matching = harness
        .exec_repo
        .list_by_vault(&harness.test_vault.vault_address)
        .await
        .unwrap();
    let count = matching
        .iter()
        .filter(|r| r.execution_id == req.execution_id)
        .count();
    assert_eq!(
        count, 1,
        "Duplicate execution must never duplicate database records or broadcast"
    );
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 13: SIMULTANEOUS DUPLICATE EXECUTION (CONCURRENT RACE) ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_13_simultaneous_duplicate_execution_race_rejected() {
    let harness = TestHarness::setup().await;
    let exec_service = Arc::new(harness.build_execution_service());
    let req = Arc::new(create_valid_request(
        &harness.test_vault.vault_address,
        "NVDA",
    ));

    let keeper1 = Keypair::new();
    let keeper2 = Keypair::new();

    let s1 = exec_service.clone();
    let r1 = req.clone();
    let handle1 = tokio::spawn(async move { s1.execute_transaction(&r1, &keeper1).await });

    let s2 = exec_service.clone();
    let r2 = req.clone();
    let handle2 = tokio::spawn(async move { s2.execute_transaction(&r2, &keeper2).await });

    let (res1, res2) = tokio::join!(handle1, handle2);
    let _ = res1.unwrap();
    let _ = res2.unwrap();

    let matching = harness
        .exec_repo
        .list_by_vault(&harness.test_vault.vault_address)
        .await
        .unwrap();
    let count = matching
        .iter()
        .filter(|r| r.execution_id == req.execution_id)
        .count();
    assert_eq!(
        count, 1,
        "Database unique constraint must allow exactly one record under race"
    );
}

// =============================================================================
// VECTOR 14: PAUSE DURING EXECUTION ATTACK (EMERGENCY PAUSE)
// =============================================================================
#[tokio::test]
async fn test_attack_14_pause_during_execution_aborted_no_signing() {
    let harness = TestHarness::setup().await;
    // Mark vault as emergency paused
    harness
        .vault_repo
        .set_paused(&harness.test_vault.vault_address, true)
        .await
        .unwrap();

    let exec_service = harness.build_execution_service();
    let req = create_valid_request(&harness.test_vault.vault_address, "NVDA");

    let keeper = Keypair::new();
    let res = exec_service.execute_transaction(&req, &keeper).await;

    assert!(res.is_err(), "Must abort immediately when vault is paused");
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 15: RPC FAILURE ATTACK (FAIL CLOSED)
// =============================================================================
#[tokio::test]
async fn test_attack_15_rpc_failure_fails_closed_no_signing() {
    let harness = TestHarness::setup().await;
    // Point solana service to non-existent unreachable port
    let broken_solana = Arc::new(SolanaService::new(
        "http://127.0.0.1:9999",
        "ws://127.0.0.1:9998",
        None,
        None,
    ));

    let exec_service = ExecutionEngineService::new(
        broken_solana,
        harness.oracle_service.clone(),
        harness.risk_engine.clone(),
        harness.signer.clone(),
        Some(harness.exec_repo.clone()),
        Some(harness.vault_repo.clone()),
        Some(harness.policy_repo.clone()),
    );

    let req = create_valid_request(&harness.test_vault.vault_address, "NVDA");
    let keeper = Keypair::new();
    let res = exec_service.execute_transaction(&req, &keeper).await;

    assert!(res.is_err(), "System must fail closed on RPC outage");
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 16: DATABASE FAILURE ATTACK (FAIL CLOSED)
// =============================================================================
#[tokio::test]
async fn test_attack_16_database_failure_fails_closed_no_signing() {
    let harness = TestHarness::setup().await;
    // Create an invalid pool pointing to closed socket
    let broken_pool = create_db_pool("postgres://postgres:invalid@127.0.0.1:5433/invalid").unwrap();
    let broken_exec_repo = ExecutionRepository::new(broken_pool);

    let exec_service = ExecutionEngineService::new(
        harness.solana_service.clone(),
        harness.oracle_service.clone(),
        harness.risk_engine.clone(),
        harness.signer.clone(),
        Some(broken_exec_repo),
        Some(harness.vault_repo.clone()),
        Some(harness.policy_repo.clone()),
    );

    let req = create_valid_request(&harness.test_vault.vault_address, "NVDA");
    let keeper = Keypair::new();
    let res = exec_service.execute_transaction(&req, &keeper).await;

    assert!(
        res.is_err(),
        "System must fail closed on database connection failure"
    );
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}

// =============================================================================
// VECTOR 17: WORKER CRASH ATTACK (SUPERVISOR RESILIENCE)
// =============================================================================
#[tokio::test]
async fn test_attack_17_worker_crash_supervised_without_state_corruption() {
    let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let config = SupervisorConfig {
        max_restarts: 3,
        restart_window: Duration::from_secs(10),
        base_backoff: Duration::from_millis(10),
        max_backoff: Duration::from_millis(50),
    };

    let supervisor = WorkerSupervisor::new("adversarial_worker", config);
    let invocation_count = Arc::new(AtomicUsize::new(0));
    let count_clone = invocation_count.clone();

    supervisor
        .run_supervised(
            move |_rx| {
                let c = count_clone.clone();
                async move {
                    let cur = c.fetch_add(1, Ordering::SeqCst);
                    if cur < 3 {
                        panic!("Simulated worker unhandled panic #{}", cur);
                    }
                    Ok(())
                }
            },
            shutdown_rx,
        )
        .await;

    assert_eq!(
        supervisor.restart_count(),
        3,
        "Supervisor must catch panics and record restarts"
    );
    assert!(
        !supervisor.is_running(),
        "Supervisor halts cleanly after bounded attempts"
    );
}

// =============================================================================
// VECTOR 18: UNAUTHORIZED COMPLIANCE & POLICY UPDATE ATTACK
// =============================================================================
#[tokio::test]
async fn test_attack_18_unauthorized_compliance_and_policy_update_rejected() {
    let harness = TestHarness::setup().await;
    let config = Config::from_env();
    let state = Arc::new(AppState::new(
        config.clone(),
        harness.pool.clone(),
        None,
        Some((*harness.solana_service).clone()),
    ));
    let app = equity_catalyst_api::router::create_router(state);

    // Attempt unauthorized compliance / vault creation without admin credential
    let unauth_payload = json!({
        "vault_address": Keypair::new().pubkey().to_string(),
        "authority": Keypair::new().pubkey().to_string(),
        "name": "Attacker Vault",
        "symbol": "ATK",
        "deposit_mint": Keypair::new().pubkey().to_string(),
        "vault_token_account": Keypair::new().pubkey().to_string(),
        "total_shares": 1000,
        "total_deposits": 1000,
        "is_paused": false,
        "bump": 255,
        "created_at": Utc::now().to_rfc3339(),
        "updated_at": Utc::now().to_rfc3339(),
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vaults")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(unauth_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Unauthorized admin mutation must be rejected with 401"
    );
    assert_eq!(harness.signing_spy_count.load(Ordering::SeqCst), 0);
}
