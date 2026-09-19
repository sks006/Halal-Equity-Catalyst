use chrono::Utc;
use equity_catalyst_api::{
    config::Config,
    create_db_pool,
    engines::{
        decision_engine::{DecisionEngine, ExecutionSigner},
        policy_engine::{PolicyEngine, PolicyRule},
        risk_engine::{RiskAssessment, RiskEngine},
    },
    models::{EventModel, PolicyModel, PortfolioModel, VaultModel},
    repositories::VaultRepository,
    services::VaultService,
};
use equity_catalyst_shared::{allocation::RebalanceTrade, types::SignalType};
use uuid::Uuid;

fn setup_test_vault_service() -> VaultService {
    let config = Config::from_env();
    let pool = create_db_pool(&config.database_url).expect("Failed to connect to db");
    let repo = VaultRepository::new(pool);
    VaultService::new(repo)
}

#[tokio::test]
async fn test_vault_service_lifecycle_and_accounting() {
    let service = setup_test_vault_service();
    let random_suffix = &Uuid::new_v4().to_string()[..8];
    let address = format!("VaultSvc{}", random_suffix);

    let vault = VaultModel {
        vault_address: address.clone(),
        authority: format!("Auth{}", random_suffix),
        name: "Liquid Tech Vault".to_string(),
        symbol: "EC-TECH".to_string(),
        deposit_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        vault_token_account: "TokenAccTech11111111111111111111111111111111".to_string(),
        total_shares: 0,
        total_deposits: 0,
        is_paused: false,
        bump: 253,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // 1. Create vault
    service
        .create_vault(&vault)
        .await
        .expect("Failed to create vault");

    // 2. Read vault
    let fetched = service
        .get_vault(&address)
        .await
        .expect("Failed to get vault");
    assert_eq!(fetched.total_shares, 0);

    // 3. Track deposit
    let post_deposit = service
        .track_deposit(&address, 100_000, 100_000)
        .await
        .expect("Failed to track deposit");
    assert_eq!(post_deposit.total_deposits, 100_000);
    assert_eq!(post_deposit.total_shares, 100_000);

    // 4. Track withdrawal
    let post_withdrawal = service
        .track_withdrawal(&address, 25_000, 25_000)
        .await
        .expect("Failed to track withdrawal");
    assert_eq!(post_withdrawal.total_deposits, 75_000);
    assert_eq!(post_withdrawal.total_shares, 75_000);

    // 5. Test emergency pause enforcement
    service
        .set_paused(&address, true)
        .await
        .expect("Failed to pause");
    let paused_vault = service.get_vault(&address).await.unwrap();
    assert!(paused_vault.is_paused);

    // Deposit should be rejected when paused
    let deposit_err = service.track_deposit(&address, 10_000, 10_000).await;
    assert!(deposit_err.is_err());

    // Withdrawal should be rejected when paused
    let withdraw_err = service.track_withdrawal(&address, 10_000, 10_000).await;
    assert!(withdraw_err.is_err());
}

#[test]
fn test_policy_engine_event_to_allocation_flow() {
    let engine = PolicyEngine::new();

    let policy = PolicyModel {
        policy_address: "Policy11111111111111111111111111111111111111".to_string(),
        vault_address: "Vault111111111111111111111111111111111111111".to_string(),
        authority: "Auth1111111111111111111111111111111111111111".to_string(),
        min_cash_bps: 1_000,
        max_position_bps: 5_000,
        stop_loss_bps: 500,
        take_profit_bps: 1_500,
        rebalance_threshold_bps: 200,
        is_active: true,
        bump: 254,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let positions = vec![
        PortfolioModel {
            portfolio_id: Uuid::new_v4(),
            vault_address: policy.vault_address.clone(),
            asset_symbol: "NVDA".to_string(),
            asset_mint: "NVDA_MINT".to_string(),
            amount: 1_000,
            entry_price_usd: 120.0,
            current_price_usd: 120.0,
            current_value_usd: 120_000.0,
            target_weight_bps: 3_500, // 35.00%
            current_weight_bps: 3_500,
            last_rebalanced_at: None,
            updated_at: Utc::now(),
        },
        PortfolioModel {
            portfolio_id: Uuid::new_v4(),
            vault_address: policy.vault_address.clone(),
            asset_symbol: "USDC".to_string(),
            asset_mint: "USDC_MINT".to_string(),
            amount: 222_857,
            entry_price_usd: 1.0,
            current_price_usd: 1.0,
            current_value_usd: 222_857.0,
            target_weight_bps: 6_500, // 65.00%
            current_weight_bps: 6_500,
            last_rebalanced_at: None,
            updated_at: Utc::now(),
        },
    ];

    let total_value = 342_857;

    // Test Bullish Earnings Event
    let bullish_event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some(policy.vault_address.clone()),
        event_type: "EARNINGS_RELEASE".to_string(),
        source: "pyth_oracle".to_string(),
        sentiment_score: Some(0.85),
        payload: serde_json::json!({ "symbol": "NVDA", "revenue_beat": 0.12 }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };

    let result = engine
        .evaluate(&bullish_event, &policy, &positions, total_value)
        .expect("Policy evaluation failed");

    assert!(matches!(result.rule, PolicyRule::EarningsBeat { .. }));
    assert_eq!(result.signal.signal_type, SignalType::Bullish);
    assert_eq!(result.signal.weight_delta_bps, 500);

    // Target allocation should have expanded NVDA target weight to 4,000 bps (40%)
    let nvda_target = result
        .target_allocation
        .weights
        .iter()
        .find(|w| w.symbol == "NVDA")
        .unwrap();
    assert_eq!(nvda_target.target_weight.0, 4_000);
}

#[test]
fn test_risk_engine_defense_lines() {
    let engine = RiskEngine::new();

    let policy = PolicyModel {
        policy_address: "P1".to_string(),
        vault_address: "V1".to_string(),
        authority: "A1".to_string(),
        min_cash_bps: 1_000,     // 10% minimum cash reserve
        max_position_bps: 3_000, // 30% max position cap
        stop_loss_bps: 500,      // 5% stop loss
        take_profit_bps: 1_500,
        rebalance_threshold_bps: 200,
        is_active: true,
        bump: 254,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let positions = vec![PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: "V1".to_string(),
        asset_symbol: "NVDA".to_string(),
        asset_mint: "NVDA_MINT_11111111111111111111111111111111111".to_string(),
        amount: 250,
        entry_price_usd: 100.0,
        current_price_usd: 100.0,
        current_value_usd: 25_000.0,
        target_weight_bps: 2_500,
        current_weight_bps: 2_500,
        last_rebalanced_at: None,
        updated_at: Utc::now(),
    }];

    let total_portfolio_usd = 100_000;

    // 1. Compliant trade: Buy $4,000 NVDA (total $29,000 <= 30% max position) with ample cash ($30,000)
    let compliant_trades = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: true,
        current_value: 25_000,
        target_value: 29_000,
        trade_value: 4_000,
        drift_bps: equity_catalyst_shared::types::BasisPoints(400),
    }];
    let assessment = engine.evaluate_proposed_trades(
        &compliant_trades,
        &positions,
        &policy,
        total_portfolio_usd,
        30_000,
    );
    assert_eq!(assessment, RiskAssessment::Approved);

    // 2. Exposure breach: Buy $10,000 NVDA (total $35,000 > 30% max position)
    let excessive_trades = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: true,
        current_value: 25_000,
        target_value: 35_000,
        trade_value: 10_000,
        drift_bps: equity_catalyst_shared::types::BasisPoints(1_000),
    }];
    let assessment_breach = engine.evaluate_proposed_trades(
        &excessive_trades,
        &positions,
        &policy,
        total_portfolio_usd,
        30_000,
    );
    assert!(matches!(assessment_breach, RiskAssessment::Rejected { .. }));

    // 3. Cash reserve breach: Available cash $12,000 is insufficient for $4,000 outlay + $10,000 required reserve
    let assessment_cash = engine.evaluate_proposed_trades(
        &compliant_trades,
        &positions,
        &policy,
        total_portfolio_usd,
        12_000,
    );
    assert!(matches!(assessment_cash, RiskAssessment::Rejected { .. }));
}

#[test]
fn test_decision_engine_end_to_end_pipeline() {
    let signer = ExecutionSigner::load_or_generate("~/.config/solana/id.json");
    let decision_engine = DecisionEngine::new(signer.clone());

    let vault = VaultModel {
        vault_address: "VaultABC11111111111111111111111111111111111".to_string(),
        authority: "AuthABC111111111111111111111111111111111111".to_string(),
        name: "Test Alpha Vault".to_string(),
        symbol: "EC-ALPHA".to_string(),
        deposit_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        vault_token_account: "TokenAccAlpha111111111111111111111111111111".to_string(),
        total_shares: 500_000,
        total_deposits: 500_000,
        is_paused: false,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let policy = PolicyModel {
        policy_address: "PolicyABC1111111111111111111111111111111111".to_string(),
        vault_address: vault.vault_address.clone(),
        authority: vault.authority.clone(),
        min_cash_bps: 1_000,
        max_position_bps: 5_000, // 50% max position
        stop_loss_bps: 500,
        take_profit_bps: 1_500,
        rebalance_threshold_bps: 200,
        is_active: true,
        bump: 254,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let positions = vec![
        PortfolioModel {
            portfolio_id: Uuid::new_v4(),
            vault_address: vault.vault_address.clone(),
            asset_symbol: "NVDA".to_string(),
            asset_mint: "NVDA_MINT_ABC".to_string(),
            amount: 1_000,
            entry_price_usd: 120.0,
            current_price_usd: 120.0,
            current_value_usd: 120_000.0,
            target_weight_bps: 2_400,
            current_weight_bps: 2_400,
            last_rebalanced_at: None,
            updated_at: Utc::now(),
        },
        PortfolioModel {
            portfolio_id: Uuid::new_v4(),
            vault_address: vault.vault_address.clone(),
            asset_symbol: "USDC".to_string(),
            asset_mint: "USDC_MINT_ABC".to_string(),
            amount: 380_000,
            entry_price_usd: 1.0,
            current_price_usd: 1.0,
            current_value_usd: 380_000.0,
            target_weight_bps: 7_600,
            current_weight_bps: 7_600,
            last_rebalanced_at: None,
            updated_at: Utc::now(),
        },
    ];

    let event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some(vault.vault_address.clone()),
        event_type: "EARNINGS_ANNOUNCEMENT".to_string(),
        source: "pyth_network".to_string(),
        sentiment_score: Some(0.92),
        payload: serde_json::json!({ "symbol": "NVDA", "beat_pct": 14.5 }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };

    let total_value = 500_000;
    let available_cash = 380_000;
    let request = decision_engine
        .process_event(
            &event,
            &vault,
            &policy,
            &positions,
            total_value,
            available_cash,
        )
        .expect("Decision pipeline failed");

    assert!(request.approved);
    assert_eq!(request.action, "BUY");
    assert!(!request.trades.is_empty());
    assert_eq!(request.vault_address, vault.vault_address);

    // Sign the execution request
    let signature = signer.sign_decision(&request.decision_id);
    assert!(signature.starts_with("sig_"));
}
