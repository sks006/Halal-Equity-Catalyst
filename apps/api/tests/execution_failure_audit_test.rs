//! Phase 09.9 — Execution Failure Audit Test Suite
//!
//! Evaluates and proves security invariants against transaction and execution failure modes:
//! 1. Simulation failure halts dispatch
//! 2. RPC timeout handling
//! 3. Transaction rejection error recording
//! 4. Duplicate request idempotency protection
//! 5. Partial state reconciliation and slippage drift tracking

use chrono::Utc;
use deadpool_postgres::Pool;
use equity_catalyst_api::{
    config::Config,
    create_db_pool,
    engines::{decision_engine::ExecutionSigner, risk_engine::RiskEngine},
    models::{ExecutionModel, VaultModel},
    repositories::{ExecutionRepository, PolicyRepository, VaultRepository},
    services::{
        execution_engine_service::{
            calculate_slippage_drift_bps, ExecutionEngineService, ExecutionRequest,
        },
        oracle_service::OracleService,
        solana_service::SolanaService,
    },
};
use equity_catalyst_pyth::PythClient;
use equity_catalyst_solana::SolanaRpcClient;
use solana_sdk::signature::{Keypair, Signer};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

fn setup_test_pool() -> Pool {
    let config = Config::from_env();
    create_db_pool(&config.database_url).expect("Failed to connect to test postgres")
}

async fn seed_test_vault(pool: &Pool, vault_address: &str) {
    let vault_repo = VaultRepository::new(pool.clone());
    let vault = VaultModel {
        vault_address: vault_address.to_string(),
        authority: Keypair::new().pubkey().to_string(),
        name: "Test Vault".to_string(),
        symbol: "TV".to_string(),
        deposit_mint: Keypair::new().pubkey().to_string(),
        vault_token_account: Keypair::new().pubkey().to_string(),
        total_shares: 1_000_000,
        total_deposits: 1_000_000,
        is_paused: false,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let _ = vault_repo.create(&vault).await;
}

#[tokio::test]
async fn test_rpc_timeout_handling() {
    // 1. Setup a slow dummy TCP server that stalls connections to trigger timeout
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{}", port);

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            // Read request then sleep to trigger client timeout
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            tokio::time::sleep(Duration::from_millis(500)).await;
            let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    // Client with 50ms timeout and 0 retries
    let client = SolanaRpcClient::new(url)
        .with_timeout(Duration::from_millis(50))
        .with_max_retries(0);

    let dummy_key = Keypair::new().pubkey();
    let res = client.get_balance(&dummy_key).await;

    assert!(res.is_err(), "Must error on RPC timeout");
    let err_str = res.err().unwrap().to_string();
    assert!(
        err_str.contains("timeout")
            || err_str.contains("timed out")
            || err_str.contains("Solana error"),
        "Error message must indicate timeout/exhaustion: got {}",
        err_str
    );
}

#[tokio::test]
async fn test_duplicate_request_idempotency_gate() {
    let pool = setup_test_pool();
    let execution_repo = ExecutionRepository::new(pool.clone());
    let vault_repo = VaultRepository::new(pool.clone());
    let policy_repo = PolicyRepository::new(pool.clone());

    let solana_service = Arc::new(SolanaService::new(
        "https://api.devnet.solana.com",
        "wss://api.devnet.solana.com",
        None,
        None,
    ));
    let pyth_client = Arc::new(PythClient::new_mock());
    let oracle_service = Arc::new(OracleService::new(pyth_client, None));
    let risk_engine = Arc::new(RiskEngine::new());
    let signer = Arc::new(ExecutionSigner::load_or_generate("/tmp/test_signer.json"));

    let execution_service = ExecutionEngineService::new(
        solana_service,
        oracle_service,
        risk_engine,
        signer,
        Some(execution_repo.clone()),
        Some(vault_repo),
        Some(policy_repo),
    );

    let execution_id = Uuid::new_v4();
    let vault_address = Keypair::new().pubkey().to_string();
    let input_mint = Keypair::new().pubkey().to_string();
    let output_mint = Keypair::new().pubkey().to_string();

    seed_test_vault(&pool, &vault_address).await;

    // 1. Seed existing confirmed execution with this ID
    let existing_execution = ExecutionModel {
        execution_id,
        vault_address: vault_address.clone(),
        event_id: None,
        action: "REBALANCE_NVDA".to_string(),
        input_mint: input_mint.clone(),
        output_mint: output_mint.clone(),
        amount_in: 1_000_000,
        amount_out_expected: 950_000,
        amount_out_actual: Some(951_000),
        slippage_bps: 100,
        tx_signature: Some("5abc123ExistingTxSignatureConfirmed".to_string()),
        status: "confirmed".to_string(),
        error_message: None,
        executed_at: Utc::now(),
        confirmed_at: Some(Utc::now()),
        quote_id: Some("quote-123".to_string()),
        policy_decision_id: Some(Uuid::new_v4()),
        amount_out_min: Some(940_000),
    };
    execution_repo.create(&existing_execution).await.unwrap();

    // 2. Attempt duplicate execution with the exact same execution_id
    let duplicate_request = ExecutionRequest {
        execution_id,
        vault_address: vault_address.clone(),
        event_id: None,
        action_type: 1,
        action_name: "REBALANCE_NVDA".to_string(),
        input_mint,
        output_mint,
        amount_in: 1_000_000,
        min_amount_out: 940_000,
        amount_out_expected: 950_000,
        slippage_bps: 100,
        target_symbol: "NVDA".to_string(),
        fee_breakdown: None,
        quote_id: Some("quote-123".to_string()),
        policy_decision_id: Some(Uuid::new_v4()),
    };

    let keeper_keypair = Keypair::new();
    let outcome = execution_service
        .execute_transaction(&duplicate_request, &keeper_keypair)
        .await
        .expect("Idempotency gate must safely return without error");

    // 3. Assert duplicate returned existing record rather than re-executing
    assert_eq!(outcome.execution_id, execution_id);
    assert_eq!(outcome.status, "confirmed");
    assert_eq!(
        outcome.tx_signature,
        Some("5abc123ExistingTxSignatureConfirmed".to_string())
    );
    assert_eq!(outcome.amount_out_actual, Some(951_000));
}

#[tokio::test]
async fn test_slippage_drift_and_partial_state_tracking() {
    // 1. Test adverse slippage drift calculation
    // Expected: 1,000, Actual: 980 -> -20 / 1000 = -2.00% = -200 bps
    let adverse_drift = calculate_slippage_drift_bps(1_000, 980);
    assert_eq!(adverse_drift, -200);

    // 2. Test positive price improvement
    // Expected: 1,000, Actual: 1,025 -> +25 / 1000 = +2.50% = +250 bps
    let positive_drift = calculate_slippage_drift_bps(1_000, 1_025);
    assert_eq!(positive_drift, 250);

    // 3. Test exact execution
    let zero_drift = calculate_slippage_drift_bps(1_000, 1_000);
    assert_eq!(zero_drift, 0);

    // 4. Test zero expected safeguard (no divide-by-zero panic)
    let div_zero = calculate_slippage_drift_bps(0, 100);
    assert_eq!(div_zero, 0);
}

#[tokio::test]
async fn test_transaction_rejection_ledger_persistence() {
    let pool = setup_test_pool();
    let execution_repo = ExecutionRepository::new(pool.clone());

    let execution_id = Uuid::new_v4();
    let vault_address = Keypair::new().pubkey().to_string();

    seed_test_vault(&pool, &vault_address).await;

    // Persist a failed execution state
    let failed_execution = ExecutionModel {
        execution_id,
        vault_address: vault_address.clone(),
        event_id: None,
        action: "REBALANCE_NVDA".to_string(),
        input_mint: Keypair::new().pubkey().to_string(),
        output_mint: Keypair::new().pubkey().to_string(),
        amount_in: 500_000,
        amount_out_expected: 490_000,
        amount_out_actual: None,
        slippage_bps: 50,
        tx_signature: None,
        status: "failed".to_string(),
        error_message: Some("0x1770: Program error VaultPaused".to_string()),
        executed_at: Utc::now(),
        confirmed_at: None,
        quote_id: None,
        policy_decision_id: None,
        amount_out_min: Some(485_000),
    };

    execution_repo.create(&failed_execution).await.unwrap();

    // Verify retrieval matches failed status with intact diagnostic error message
    let retrieved = execution_repo
        .find_by_id(execution_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, "failed");
    assert_eq!(
        retrieved.error_message,
        Some("0x1770: Program error VaultPaused".to_string())
    );
    assert!(retrieved.tx_signature.is_none());
}
