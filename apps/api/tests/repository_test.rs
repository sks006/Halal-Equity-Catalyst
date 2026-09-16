use chrono::Utc;
use equity_catalyst_api::{
    config::Config,
    create_db_pool,
    models::{EventModel, ExecutionModel, PolicyModel, PortfolioModel, VaultModel},
    repositories::{
        EventRepository, ExecutionRepository, PolicyRepository, PortfolioRepository,
        VaultRepository,
    },
};
use uuid::Uuid;

fn setup_repositories() -> (
    VaultRepository,
    PolicyRepository,
    EventRepository,
    ExecutionRepository,
    PortfolioRepository,
) {
    let config = Config::from_env();
    let pool = create_db_pool(&config.database_url).expect("Failed to connect to database");
    (
        VaultRepository::new(pool.clone()),
        PolicyRepository::new(pool.clone()),
        EventRepository::new(pool.clone()),
        ExecutionRepository::new(pool.clone()),
        PortfolioRepository::new(pool),
    )
}

#[tokio::test]
async fn test_full_repository_lifecycle() {
    let (vault_repo, policy_repo, event_repo, execution_repo, portfolio_repo) =
        setup_repositories();

    let random_suffix = &Uuid::new_v4().to_string()[..8];
    let vault_address = format!("TestVault{}", random_suffix);
    let authority = format!("Auth{}", random_suffix);
    let policy_address = format!("TestPolicy{}", random_suffix);

    // 1. Test VaultRepository
    let vault = VaultModel {
        vault_address: vault_address.clone(),
        authority: authority.clone(),
        name: "Test NVDA Vault".to_string(),
        symbol: "EC-NVDA".to_string(),
        deposit_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        vault_token_account: "TokenAcc1111111111111111111111111111111111".to_string(),
        total_shares: 1_000_000,
        total_deposits: 1_000_000,
        is_paused: false,
        bump: 254,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created_vault = vault_repo
        .create(&vault)
        .await
        .expect("Failed to create vault");
    assert_eq!(created_vault.vault_address, vault_address);
    assert_eq!(created_vault.name, "Test NVDA Vault");

    let fetched_vault = vault_repo
        .find_by_address(&vault_address)
        .await
        .expect("Failed to fetch vault")
        .expect("Vault not found");
    assert_eq!(fetched_vault.total_shares, 1_000_000);

    vault_repo
        .update_totals(&vault_address, 2_500_000, 2_500_000)
        .await
        .expect("Failed to update totals");

    vault_repo
        .set_paused(&vault_address, true)
        .await
        .expect("Failed to set paused");

    let updated_vault = vault_repo
        .find_by_address(&vault_address)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_vault.total_shares, 2_500_000);
    assert!(updated_vault.is_paused);

    // 2. Test PolicyRepository
    let policy = PolicyModel {
        policy_address: policy_address.clone(),
        vault_address: vault_address.clone(),
        authority: authority.clone(),
        max_ltv_bps: 7_500,
        max_position_bps: 2_500,
        stop_loss_bps: 500,
        take_profit_bps: 1_500,
        rebalance_threshold_bps: 200,
        is_active: true,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let upserted_policy = policy_repo
        .upsert(&policy)
        .await
        .expect("Failed to upsert policy");
    assert_eq!(upserted_policy.policy_address, policy_address);
    assert_eq!(upserted_policy.max_ltv_bps, 7_500);

    let fetched_policy = policy_repo
        .find_by_vault(&vault_address)
        .await
        .expect("Failed to find policy")
        .expect("Policy not found");
    assert_eq!(fetched_policy.max_position_bps, 2_500);

    // 3. Test EventRepository
    let event_id = Uuid::new_v4();
    let event = EventModel {
        event_id,
        vault_address: Some(vault_address.clone()),
        event_type: "EARNINGS_BEAT".to_string(),
        source: "pyth_oracle".to_string(),
        sentiment_score: Some(0.85),
        payload: serde_json::json!({ "eps_actual": 0.68, "eps_estimate": 0.64 }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };

    let created_event = event_repo
        .create(&event)
        .await
        .expect("Failed to create event");
    assert_eq!(created_event.event_id, event_id);
    assert_eq!(created_event.status, "PENDING");

    let pending_events = event_repo
        .find_pending()
        .await
        .expect("Failed to find pending events");
    assert!(pending_events.iter().any(|e| e.event_id == event_id));

    event_repo
        .update_status(event_id, "PROCESSED", Some(Utc::now()))
        .await
        .expect("Failed to update event status");

    let processed_event = event_repo.find_by_id(event_id).await.unwrap().unwrap();
    assert_eq!(processed_event.status, "PROCESSED");
    assert!(processed_event.processed_at.is_some());

    // 4. Test ExecutionRepository
    let execution_id = Uuid::new_v4();
    let execution = ExecutionModel {
        execution_id,
        vault_address: vault_address.clone(),
        event_id: Some(event_id),
        action: "BUY".to_string(),
        input_mint: "USDC_MINT_11111111111111111111111111111111".to_string(),
        output_mint: "NVDA_MINT_11111111111111111111111111111111".to_string(),
        amount_in: 50_000_000,
        amount_out_expected: 400_000,
        amount_out_actual: None,
        slippage_bps: 50,
        tx_signature: None,
        status: "PENDING".to_string(),
        error_message: None,
        executed_at: Utc::now(),
        confirmed_at: None,
    };

    let created_exec = execution_repo
        .create(&execution)
        .await
        .expect("Failed to create execution");
    assert_eq!(created_exec.execution_id, execution_id);

    execution_repo
        .update_status(
            execution_id,
            "CONFIRMED",
            Some("5J3m...sig111111111111111111111111111111111111111111111111111111111111111111111111"),
            Some(399_850),
            None,
            Some(Utc::now()),
        )
        .await
        .expect("Failed to update execution");

    let confirmed_exec = execution_repo
        .find_by_id(execution_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(confirmed_exec.status, "CONFIRMED");
    assert_eq!(confirmed_exec.amount_out_actual, Some(399_850));

    // 5. Test PortfolioRepository
    let position = PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: vault_address.clone(),
        asset_symbol: "NVDA".to_string(),
        asset_mint: "NVDA_MINT_11111111111111111111111111111111".to_string(),
        amount: 1_200_000,
        entry_price_usd: 120.50,
        current_price_usd: 128.75,
        current_value_usd: 154_500.0,
        target_weight_bps: 4_000,
        current_weight_bps: 4_100,
        last_rebalanced_at: Some(Utc::now()),
        updated_at: Utc::now(),
    };

    let saved_pos = portfolio_repo
        .upsert_position(&position)
        .await
        .expect("Failed to upsert portfolio position");
    assert_eq!(saved_pos.asset_symbol, "NVDA");
    assert_eq!(saved_pos.current_price_usd, 128.75);

    let portfolio_list = portfolio_repo
        .list_by_vault(&vault_address)
        .await
        .expect("Failed to list portfolio positions");
    assert_eq!(portfolio_list.len(), 1);
    assert_eq!(portfolio_list[0].asset_symbol, "NVDA");
}
