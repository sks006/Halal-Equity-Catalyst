use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use borsh::BorshSerialize;
use chrono::Utc;
use deadpool_postgres::Pool;
use equity_catalyst_api::{
    config::Config,
    create_db_pool,
    engines::decision_engine::{DecisionEngine, ExecutionSigner},
    models::{EventModel, PolicyModel, PortfolioModel, VaultModel},
    repositories::{
        event_repository::EventRepository,
        execution_repository::ExecutionRepository,
        policy_repository::PolicyRepository,
        portfolio_repository::PortfolioRepository,
        vault_repository::VaultRepository,
    },
    services::SolanaService,
    workers::{EventListener, PolicyWorker},
};
use equity_catalyst_solana::{
    accounts::{compute_event_discriminator, DepositEvent},
    websocket::LogsNotification,
};
use serde_json::json;
use solana_sdk::signature::{Keypair, Signer};
use uuid::Uuid;

fn setup_test_pool() -> Pool {
    let config = Config::from_env();
    create_db_pool(&config.database_url).expect("Failed to connect to test postgres")
}

fn setup_test_redis() -> Option<redis::Client> {
    let config = Config::from_env();
    redis::Client::open(config.redis_url.as_str()).ok()
}

#[tokio::test]
async fn test_step29_event_listener_ingestion_and_queuing() {
    let pool = setup_test_pool();
    let redis_client = setup_test_redis();
    let event_repo = EventRepository::new(pool.clone());
    let solana_service = SolanaService::new(
        "https://api.devnet.solana.com",
        "wss://api.devnet.solana.com",
        None,
        None,
    );

    let test_queue = format!("test:events:queue:{}", Uuid::new_v4());
    let listener = EventListener::new(solana_service, event_repo.clone(), redis_client)
        .with_queue_key(&test_queue);

    // Construct simulated Anchor DepositEvent log
    let vault_pda = Keypair::new().pubkey();
    let user_pubkey = Keypair::new().pubkey();

    // Ensure vault exists in DB to satisfy foreign key constraint
    let vault_repo = VaultRepository::new(pool.clone());
    let vault_model = VaultModel {
        vault_address: vault_pda.to_string(),
        authority: Keypair::new().pubkey().to_string(),
        name: "Step 29 Vault".to_string(),
        symbol: "S29".to_string(),
        deposit_mint: Keypair::new().pubkey().to_string(),
        vault_token_account: Keypair::new().pubkey().to_string(),
        total_shares: 0,
        total_deposits: 0,
        is_paused: false,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    vault_repo.create(&vault_model).await.expect("Failed to seed vault for event");

    let deposit_event = DepositEvent {
        vault: vault_pda,
        user: user_pubkey,
        amount: 2_500_000,
        shares: 2_500_000,
        timestamp: Utc::now().timestamp(),
    };

    let mut event_bytes = Vec::new();
    event_bytes.extend_from_slice(&compute_event_discriminator("Deposit"));
    deposit_event.serialize(&mut event_bytes).unwrap();

    let b64_event = BASE64.encode(&event_bytes);
    let log_line = format!("Program data: {}", b64_event);

    let notification = LogsNotification {
        signature: "5xyzSimulatedSolanaSignature111111111111111111".to_string(),
        err: None,
        logs: vec![
            "Program 8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH invoke [1]".to_string(),
            "Program log: Instruction: Deposit".to_string(),
            log_line,
            "Program 8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH success".to_string(),
        ],
    };

    // Process notification
    let maybe_event = listener
        .process_notification(&notification)
        .await
        .expect("Failed to process logs notification");

    assert!(maybe_event.is_some());
    let event = maybe_event.unwrap();

    // Verify parsed data
    assert_eq!(event.event_type, "DEPOSIT");
    assert_eq!(event.vault_address, Some(vault_pda.to_string()));
    assert_eq!(event.status, "PENDING");
    assert_eq!(event.source, "solana_websocket");

    // Verify persisted in PostgreSQL
    let queried = event_repo
        .find_by_id(event.event_id)
        .await
        .expect("Failed to find event in Postgres")
        .expect("Event missing from Postgres");
    assert_eq!(queried.event_id, event.event_id);
    assert_eq!(queried.status, "PENDING");
}

#[tokio::test]
async fn test_step30_policy_worker_evaluation_and_dry_run_logging() {
    let pool = setup_test_pool();
    let redis_client = setup_test_redis();

    let vault_repo = VaultRepository::new(pool.clone());
    let policy_repo = PolicyRepository::new(pool.clone());
    let portfolio_repo = PortfolioRepository::new(pool.clone());
    let event_repo = EventRepository::new(pool.clone());
    let execution_repo = ExecutionRepository::new(pool.clone());

    let signer = ExecutionSigner::load_or_generate("~/.config/solana/id.json");
    let decision_engine = DecisionEngine::new(signer);

    let test_queue = format!("test:policy:queue:{}", Uuid::new_v4());
    let worker = PolicyWorker::new(
        decision_engine,
        vault_repo.clone(),
        policy_repo.clone(),
        portfolio_repo.clone(),
        event_repo.clone(),
        execution_repo.clone(),
        redis_client.clone(),
    )
    .with_queue_key(&test_queue);

    // 1. Seed test Vault in Postgres
    let vault_address = format!("TestVault_{}", Uuid::new_v4().simple());
    let authority = Keypair::new().pubkey().to_string();
    let deposit_mint = Keypair::new().pubkey().to_string();

    let vault_model = VaultModel {
        vault_address: vault_address.clone(),
        authority: authority.clone(),
        name: "Test Yield Vault".to_string(),
        symbol: "TYV".to_string(),
        deposit_mint: deposit_mint.clone(),
        vault_token_account: Keypair::new().pubkey().to_string(),
        total_shares: 100_000_000,
        total_deposits: 100_000_000,
        is_paused: false,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    vault_repo.create(&vault_model).await.expect("Failed to create vault");

    // 2. Seed test Policy in Postgres
    let policy_model = PolicyModel {
        policy_address: format!("TestPolicy_{}", Uuid::new_v4().simple()),
        vault_address: vault_address.clone(),
        authority: authority.clone(),
        max_ltv_bps: 7500,
        max_position_bps: 3000,
        stop_loss_bps: 500,
        take_profit_bps: 1500,
        rebalance_threshold_bps: 200,
        is_active: true,
        bump: 254,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    policy_repo.upsert(&policy_model).await.expect("Failed to upsert policy");

    // 3. Seed Portfolio position in Postgres
    let pos_model = PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: vault_address.clone(),
        asset_symbol: "SOL".to_string(),
        asset_mint: deposit_mint.clone(),
        amount: 100_000,
        entry_price_usd: 150.0,
        current_price_usd: 155.0,
        current_value_usd: 15_500_000.0,
        target_weight_bps: 10000,
        current_weight_bps: 10000,
        last_rebalanced_at: None,
        updated_at: Utc::now(),
    };
    portfolio_repo.upsert_position(&pos_model).await.expect("Failed to seed position");

    // 4. Create an incoming market/oracle event for this vault
    let event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some(vault_address.clone()),
        event_type: "REBALANCE_TRIGGER".to_string(),
        source: "pyth_oracle".to_string(),
        sentiment_score: Some(0.65),
        payload: json!({
            "trigger": "scheduled_drift_check",
            "volatility": "moderate"
        }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };
    let created_event = event_repo.create(&event).await.expect("Failed to create event");

    // 5. Evaluate event via PolicyWorker
    let decision = worker
        .process_single_event(&created_event)
        .await
        .expect("PolicyWorker failed to process event");

    // 6. Verify decision was generated, signed, and logged (dry-run mode)
    assert_ne!(decision.decision_id, Uuid::nil());
    assert!(!decision.action.is_empty());
    assert_eq!(decision.vault_address, vault_address);

    // 7. Verify decision was saved in Postgres executions audit table as "LOGGED" (not broadcast)
    let execution = execution_repo
        .find_by_id(decision.decision_id)
        .await
        .expect("Failed to query execution")
        .expect("Execution record was not recorded");
    assert_eq!(execution.status, "LOGGED");
    assert_eq!(execution.vault_address, vault_address);
    assert_eq!(execution.event_id, Some(created_event.event_id));

    // 8. Verify event status transitioned to "PROCESSED"
    let updated_event = event_repo
        .find_by_id(created_event.event_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_event.status, "PROCESSED");
    assert!(updated_event.processed_at.is_some());
}

#[tokio::test]
async fn test_end_to_end_event_queue_to_policy_worker_flow() {
    let pool = setup_test_pool();
    let redis_client = setup_test_redis();

    let vault_repo = VaultRepository::new(pool.clone());
    let policy_repo = PolicyRepository::new(pool.clone());
    let portfolio_repo = PortfolioRepository::new(pool.clone());
    let event_repo = EventRepository::new(pool.clone());
    let execution_repo = ExecutionRepository::new(pool.clone());

    let shared_queue = format!("test:e2e:queue:{}", Uuid::new_v4());

    // 1. Seed Vault
    let pubkey_vault = Keypair::new().pubkey();
    let vault_address = pubkey_vault.to_string();
    let authority = Keypair::new().pubkey().to_string();
    let deposit_mint = Keypair::new().pubkey().to_string();

    let vault_model = VaultModel {
        vault_address: vault_address.clone(),
        authority: authority.clone(),
        name: "E2E Yield Vault".to_string(),
        symbol: "E2E".to_string(),
        deposit_mint: deposit_mint.clone(),
        vault_token_account: Keypair::new().pubkey().to_string(),
        total_shares: 50_000_000,
        total_deposits: 50_000_000,
        is_paused: false,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    vault_repo.create(&vault_model).await.expect("Failed to seed vault");

    // 2. Seed Policy
    let policy_model = PolicyModel {
        policy_address: format!("E2EPolicy_{}", Uuid::new_v4().simple()),
        vault_address: vault_address.clone(),
        authority: authority.clone(),
        max_ltv_bps: 8000,
        max_position_bps: 3500,
        stop_loss_bps: 600,
        take_profit_bps: 2000,
        rebalance_threshold_bps: 300,
        is_active: true,
        bump: 254,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    policy_repo.upsert(&policy_model).await.expect("Failed to seed policy");

    // 3. Seed Portfolio
    let pos_model = PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: vault_address.clone(),
        asset_symbol: "SOL".to_string(),
        asset_mint: deposit_mint.clone(),
        amount: 50_000,
        entry_price_usd: 140.0,
        current_price_usd: 145.0,
        current_value_usd: 7_250_000.0,
        target_weight_bps: 10000,
        current_weight_bps: 10000,
        last_rebalanced_at: None,
        updated_at: Utc::now(),
    };
    portfolio_repo.upsert_position(&pos_model).await.expect("Failed to seed portfolio");

    // 4. Initialize EventListener with shared queue
    let solana_service = SolanaService::new(
        "https://api.devnet.solana.com",
        "wss://api.devnet.solana.com",
        None,
        None,
    );
    let listener = EventListener::new(solana_service, event_repo.clone(), redis_client.clone())
        .with_queue_key(&shared_queue);

    // 5. Ingest simulated Anchor log notification
    let user_pubkey = Keypair::new().pubkey();

    let deposit_event = DepositEvent {
        vault: pubkey_vault,
        user: user_pubkey,
        amount: 5_000_000,
        shares: 5_000_000,
        timestamp: Utc::now().timestamp(),
    };

    let mut event_bytes = Vec::new();
    event_bytes.extend_from_slice(&compute_event_discriminator("Deposit"));
    deposit_event.serialize(&mut event_bytes).unwrap();

    let notification = LogsNotification {
        signature: "4xyzE2ESolanaTxSignature11111111111111111111".to_string(),
        err: None,
        logs: vec![
            format!("Program data: {}", BASE64.encode(&event_bytes)),
        ],
    };

    let ingested = listener
        .process_notification(&notification)
        .await
        .expect("Failed to ingest notification")
        .expect("Notification was not parsed as event");
    assert_eq!(ingested.status, "PENDING");

    // 6. Initialize PolicyWorker with same shared queue
    let signer = ExecutionSigner::load_or_generate("~/.config/solana/id.json");
    let decision_engine = DecisionEngine::new(signer);
    let worker = PolicyWorker::new(
        decision_engine,
        vault_repo.clone(),
        policy_repo.clone(),
        portfolio_repo.clone(),
        event_repo.clone(),
        execution_repo.clone(),
        redis_client.clone(),
    )
    .with_queue_key(&shared_queue);

    // 7. Worker polls queue and processes event end-to-end
    let maybe_decision = worker
        .poll_and_process_next()
        .await
        .expect("Worker poll and process failed");

    assert!(maybe_decision.is_some());
    let decision = maybe_decision.unwrap();
    assert_eq!(decision.vault_address, vault_address);

    // 8. Verify execution record in Postgres with status 'LOGGED'
    let exec = execution_repo
        .find_by_id(decision.decision_id)
        .await
        .unwrap()
        .expect("Execution record missing");
    assert_eq!(exec.status, "LOGGED");

    // 9. Verify event transitioned to 'PROCESSED'
    let processed_event = event_repo
        .find_by_id(ingested.event_id)
        .await
        .unwrap()
        .expect("Event missing");
    assert_eq!(processed_event.status, "PROCESSED");
}

