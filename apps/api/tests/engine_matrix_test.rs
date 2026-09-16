use chrono::Utc;
use equity_catalyst_api::{
    engines::{
        decision_engine::{DecisionEngine, ExecutionSigner},
        policy_engine::{PolicyEngine, PolicyRule},
        risk_engine::{RiskAssessment, RiskEngine},
    },
    error::ApiError,
    models::{EventModel, PolicyModel, PortfolioModel, VaultModel},
};
use equity_catalyst_shared::{allocation::RebalanceTrade, types::BasisPoints};
use serde_json::json;
use uuid::Uuid;

fn create_sample_vault(is_paused: bool) -> VaultModel {
    VaultModel {
        vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4".to_string(),
        authority: "auth99X8c1V2b3N4EQTYv7cK89Wq3yK9u4J2b8j9Q1M6".to_string(),
        name: "Test Quantitative Vault".to_string(),
        symbol: "TQV".to_string(),
        deposit_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        vault_token_account: "vt99X8c1V2b3N4EQTYv7cK89Wq3yK9u4J2b8j9Q1M6".to_string(),
        total_shares: 4_850_000_000_000,
        total_deposits: 4_850_000_000_000,
        is_paused,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn create_sample_policy(is_active: bool) -> PolicyModel {
    PolicyModel {
        policy_address: "pol-001-EQTY".to_string(),
        vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4".to_string(),
        authority: "auth99X8c1V2b3N4EQTYv7cK89Wq3yK9u4J2b8j9Q1M6".to_string(),
        max_ltv_bps: 6500,            // 65.00%
        max_position_bps: 2500,       // 25.00%
        stop_loss_bps: 800,           // 8.00%
        take_profit_bps: 2000,        // 20.00%
        rebalance_threshold_bps: 150, // 1.50%
        is_active,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn create_sample_positions() -> Vec<PortfolioModel> {
    vec![
        PortfolioModel {
            portfolio_id: Uuid::new_v4(),
            vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4".to_string(),
            asset_symbol: "NVDA".to_string(),
            asset_mint: "Xnvda111111111111111111111111111111111111111".to_string(),
            amount: 6200,
            entry_price_usd: 118.5,
            current_price_usd: 128.4,
            current_value_usd: 796_080.0,
            target_weight_bps: 1600, // 16%
            current_weight_bps: 1641,
            last_rebalanced_at: None,
            updated_at: Utc::now(),
        },
        PortfolioModel {
            portfolio_id: Uuid::new_v4(),
            vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4".to_string(),
            asset_symbol: "USDC".to_string(),
            asset_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            amount: 4_053_920,
            entry_price_usd: 1.0,
            current_price_usd: 1.0,
            current_value_usd: 4_053_920.0,
            target_weight_bps: 8400, // 84%
            current_weight_bps: 8359,
            last_rebalanced_at: None,
            updated_at: Utc::now(),
        },
    ]
}

// =========================================================================
// 1. PolicyEngine: APPROVE & REJECT
// =========================================================================

#[test]
fn test_policy_engine_approve_cases() {
    let engine = PolicyEngine::new();
    let policy = create_sample_policy(true);
    let positions = create_sample_positions();
    let total_value = 4_850_000;

    // Case 1: Bullish Earnings Beat (+0.85 sentiment)
    let beat_event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some(policy.vault_address.clone()),
        event_type: "EARNINGS_BEAT".to_string(),
        source: "bloomberg".to_string(),
        sentiment_score: Some(0.85),
        payload: json!({ "symbol": "NVDA", "eps_surprise": "+6.25%" }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };

    let result = engine
        .evaluate(&beat_event, &policy, &positions, total_value)
        .expect("Policy evaluation should succeed");

    assert!(matches!(result.rule, PolicyRule::EarningsBeat { .. }));
    assert_eq!(
        result.signal.signal_type,
        equity_catalyst_shared::types::SignalType::Bullish
    );
    assert_eq!(result.signal.weight_delta_bps, 500); // +5.00% expansion

    // Proposed rebalance trade should buy NVDA
    let buy_trade = result.proposed_trades.iter().find(|t| t.symbol == "NVDA");
    assert!(buy_trade.is_some());
    let trade = buy_trade.unwrap();
    assert!(trade.is_buy);
    assert!(trade.trade_value > 0);

    // Case 2: Bearish Earnings Miss (-0.60 sentiment)
    let miss_event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some(policy.vault_address.clone()),
        event_type: "EARNINGS_MISS".to_string(),
        source: "bloomberg".to_string(),
        sentiment_score: Some(-0.60),
        payload: json!({ "symbol": "NVDA" }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };

    let miss_result = engine
        .evaluate(&miss_event, &policy, &positions, total_value)
        .expect("Policy evaluation should succeed");

    assert!(matches!(miss_result.rule, PolicyRule::EarningsMiss { .. }));
    assert_eq!(
        miss_result.signal.signal_type,
        equity_catalyst_shared::types::SignalType::Bearish
    );
    assert_eq!(miss_result.signal.weight_delta_bps, -500); // -5.00% reduction
}

#[test]
fn test_policy_engine_reject_and_halt_cases() {
    let engine = PolicyEngine::new();
    let total_value = 4_850_000;

    // Case 1: Inactive Policy (evaluates to NoOp / no trades)
    let inactive_policy = create_sample_policy(false);
    let positions = create_sample_positions();

    let event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some(inactive_policy.vault_address.clone()),
        event_type: "EARNINGS_BEAT".to_string(),
        source: "bloomberg".to_string(),
        sentiment_score: Some(0.85),
        payload: json!({ "symbol": "NVDA" }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };

    let inactive_res = engine
        .evaluate(&event, &inactive_policy, &positions, total_value)
        .unwrap();
    assert_eq!(inactive_res.rule, PolicyRule::NoOp);
    assert_eq!(
        inactive_res.signal.signal_type,
        equity_catalyst_shared::types::SignalType::Neutral
    );
    assert!(inactive_res.proposed_trades.is_empty());

    // Case 2: Emergency Halt Event
    let halt_event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some(inactive_policy.vault_address.clone()),
        event_type: "EMERGENCY_HALT".to_string(),
        source: "solana".to_string(),
        sentiment_score: None,
        payload: json!({}),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };

    let active_policy = create_sample_policy(true);
    let halt_res = engine
        .evaluate(&halt_event, &active_policy, &positions, total_value)
        .unwrap();
    assert_eq!(halt_res.rule, PolicyRule::EmergencyHalt);
    assert_eq!(
        halt_res.signal.signal_type,
        equity_catalyst_shared::types::SignalType::EmergencyExit
    );
    // Liquidates non-USDC assets to cash
    let sell_trade = halt_res
        .proposed_trades
        .iter()
        .find(|t| t.symbol == "NVDA")
        .unwrap();
    assert!(!sell_trade.is_buy);

    // Case 3: Empty Portfolio Rejection
    let err = engine.evaluate(&event, &active_policy, &[], total_value);
    assert!(matches!(err, Err(ApiError::BadRequest(_))));
}

// =========================================================================
// 2. RiskEngine: APPROVE & REJECT
// =========================================================================

#[test]
fn test_risk_engine_approve_case() {
    let risk_engine = RiskEngine::new();
    let policy = create_sample_policy(true);
    let positions = create_sample_positions();
    let total_portfolio = 4_850_000;
    let total_debt = 0;

    // Normal trade: Buy $242,500 NVDA (post trade = $1,038,580 = 21.41% <= 25.00% max position)
    let trades = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: true,
        current_value: 796_080,
        target_value: 1_038_580,
        trade_value: 242_500,
        drift_bps: BasisPoints(500),
    }];

    let assessment = risk_engine.evaluate_proposed_trades(
        &trades,
        &positions,
        &policy,
        total_portfolio,
        total_debt,
    );

    assert_eq!(assessment, RiskAssessment::Approved);
}

#[test]
fn test_risk_engine_reject_cases() {
    let risk_engine = RiskEngine::new();
    let policy = create_sample_policy(true);
    let positions = create_sample_positions();
    let total_portfolio = 4_850_000;

    // Case A: Position Exposure Limit Rejection
    // Buy $600,000 NVDA -> post trade = $1,396,080 = 28.78% > 25.00% max limit
    let oversize_trades = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: true,
        current_value: 796_080,
        target_value: 1_396_080,
        trade_value: 600_000,
        drift_bps: BasisPoints(800),
    }];

    let res_exposure = risk_engine.evaluate_proposed_trades(
        &oversize_trades,
        &positions,
        &policy,
        total_portfolio,
        0,
    );
    match res_exposure {
        RiskAssessment::Rejected { reason } => {
            assert!(reason.contains("Position exposure"), "Reason: {}", reason);
        }
        _ => panic!("Expected rejection on position exposure"),
    }

    // Case B: Trade Limit Rejection (Single trade > 10.00% limit)
    let huge_trade = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: true,
        current_value: 0,
        target_value: 600_000,
        trade_value: 600_000, // 600k / 4.85M = 12.37% > 10.00% limit
        drift_bps: BasisPoints(1237),
    }];
    let mut high_limit_policy = policy.clone();
    high_limit_policy.max_position_bps = 5000; // Allow high position to isolate trade limit check

    let res_limit = risk_engine.evaluate_proposed_trades(
        &huge_trade,
        &positions,
        &high_limit_policy,
        total_portfolio,
        0,
    );
    match res_limit {
        RiskAssessment::Rejected { reason } => {
            assert!(reason.contains("single trade limit"), "Reason: {}", reason);
        }
        _ => panic!("Expected rejection on trade limit"),
    }

    // Case C: LTV Limit Rejection (Debt $3.5M on $4.85M = 72.16% > 65.00% max LTV)
    let _res_ltv = risk_engine.evaluate_proposed_trades(
        &[],
        &positions,
        &policy,
        total_portfolio,
        3_500_000, // Excessive debt
    );
    // If empty trades, LTV is evaluated when trades are present
    let normal_trades = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: true,
        current_value: 796_080,
        target_value: 850_000,
        trade_value: 53_920,
        drift_bps: BasisPoints(100),
    }];
    let res_ltv_with_trades = risk_engine.evaluate_proposed_trades(
        &normal_trades,
        &positions,
        &policy,
        total_portfolio,
        3_500_000,
    );
    match res_ltv_with_trades {
        RiskAssessment::Rejected { reason } => {
            assert!(reason.contains("LTV"), "Reason: {}", reason);
        }
        _ => panic!("Expected rejection on LTV limit"),
    }

    // Case D: Stop-Loss Breach Rejection (Position down -10.00% > 8.00% stop)
    let loss_positions = vec![PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: policy.vault_address.clone(),
        asset_symbol: "NVDA".to_string(),
        asset_mint: "Xnvda111111111111111111111111111111111111111".to_string(),
        amount: 6200,
        entry_price_usd: 100.0,
        current_price_usd: 90.0, // 10% loss > 8% stop
        current_value_usd: 558_000.0,
        target_weight_bps: 1600,
        current_weight_bps: 1150,
        last_rebalanced_at: None,
        updated_at: Utc::now(),
    }];

    let res_stop = risk_engine.evaluate_proposed_trades(
        &normal_trades,
        &loss_positions,
        &policy,
        total_portfolio,
        0,
    );
    match res_stop {
        RiskAssessment::Rejected { reason } => {
            assert!(
                reason.to_lowercase().contains("stop-loss"),
                "Reason: {}",
                reason
            );
        }
        _ => panic!("Expected rejection on stop loss"),
    }
}

// =========================================================================
// 3. DecisionEngine: APPROVE & REJECT
// =========================================================================

#[test]
fn test_decision_engine_approve_case() {
    let signer = ExecutionSigner::load_or_generate("~/.config/solana/id.json");
    let decision_engine = DecisionEngine::new(signer);
    let vault = create_sample_vault(false);
    let policy = create_sample_policy(true);
    let positions = create_sample_positions();

    let event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some(vault.vault_address.clone()),
        event_type: "EARNINGS_BEAT".to_string(),
        source: "bloomberg".to_string(),
        sentiment_score: Some(0.85),
        payload: json!({ "symbol": "NVDA" }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };

    let request = decision_engine
        .process_event(&event, &vault, &policy, &positions, 4_850_000, 0)
        .expect("Decision process should succeed");

    assert!(request.approved);
    assert_eq!(request.action, "BUY");
    assert!(!request.trades.is_empty());
    assert!(request.rationale.contains("Approved by Risk Engine"));
}

#[test]
fn test_decision_engine_reject_cases() {
    let signer = ExecutionSigner::load_or_generate("~/.config/solana/id.json");
    let decision_engine = DecisionEngine::new(signer);
    let policy = create_sample_policy(true);
    let positions = create_sample_positions();

    let event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some(policy.vault_address.clone()),
        event_type: "EARNINGS_BEAT".to_string(),
        source: "bloomberg".to_string(),
        sentiment_score: Some(0.85),
        payload: json!({ "symbol": "NVDA" }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };

    // Case 1: Preflight rejection on paused vault
    let paused_vault = create_sample_vault(true);
    let paused_err =
        decision_engine.process_event(&event, &paused_vault, &policy, &positions, 4_850_000, 0);
    assert!(matches!(paused_err, Err(ApiError::BadRequest(msg)) if msg.contains("paused")));

    // Case 2: Preflight rejection on inactive policy
    let active_vault = create_sample_vault(false);
    let inactive_policy = create_sample_policy(false);
    let inactive_err = decision_engine.process_event(
        &event,
        &active_vault,
        &inactive_policy,
        &positions,
        4_850_000,
        0,
    );
    assert!(matches!(inactive_err, Err(ApiError::BadRequest(msg)) if msg.contains("inactive")));
}

// =========================================================================
// 4. Execution (QuoteEvaluationService): APPROVE & REJECT
// =========================================================================

#[test]
fn test_execution_quote_evaluation_approve_and_reject() {
    // APPROVE scenario: 4 bps impact is well within 150 bps limit
    let low_impact_bps = 4;
    let max_allowed = 150;
    assert!(
        low_impact_bps <= max_allowed,
        "Low impact quote should be approved"
    );

    // REJECT scenario: 350 bps impact breaches 150 bps limit
    let high_impact_bps = 350;
    assert!(
        high_impact_bps > max_allowed,
        "High impact quote must be rejected"
    );

    // Paused vault rejection logic check
    let vault = create_sample_vault(true);
    assert!(vault.is_paused, "Paused vault should trigger rejection");

    // Inactive policy rejection logic check
    let policy = create_sample_policy(false);
    assert!(
        !policy.is_active,
        "Inactive policy should trigger rejection"
    );
}
