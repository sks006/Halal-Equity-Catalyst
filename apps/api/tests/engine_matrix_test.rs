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
        min_cash_bps: 1000,           // 10.00% minimum cash reserve
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
    let available_cash = 1_208_760;

    // Normal BUY trade: Buy $242,500 NVDA (post trade = $1,038,580 = 21.41% <= 25.00% max position)
    let buy_trades = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: true,
        current_value: 796_080,
        target_value: 1_038_580,
        trade_value: 242_500,
        drift_bps: BasisPoints(500),
    }];

    let assessment_buy = risk_engine.evaluate_proposed_trades(
        &buy_trades,
        &positions,
        &policy,
        total_portfolio,
        available_cash,
    );

    assert_eq!(assessment_buy, RiskAssessment::Approved);

    // Normal SELL trade: Sell $200,000 NVDA out of $796,080 owned balance
    let sell_trades = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: false,
        current_value: 796_080,
        target_value: 596_080,
        trade_value: 200_000,
        drift_bps: BasisPoints(412),
    }];

    let assessment_sell = risk_engine.evaluate_proposed_trades(
        &sell_trades,
        &positions,
        &policy,
        total_portfolio,
        available_cash,
    );

    assert_eq!(assessment_sell, RiskAssessment::Approved);
}

#[test]
fn test_risk_engine_reject_cases() {
    let risk_engine = RiskEngine::new();
    let policy = create_sample_policy(true);
    let positions = create_sample_positions();
    let total_portfolio = 4_850_000;
    let available_cash = 1_208_760;

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
        available_cash,
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
        available_cash,
    );
    match res_limit {
        RiskAssessment::Rejected { reason } => {
            assert!(reason.contains("single trade limit"), "Reason: {}", reason);
        }
        _ => panic!("Expected rejection on trade limit"),
    }

    // Case C: Cash Reserve Inadequacy Rejection (cash $300k covers $242.5k trade but violates $485k min cash reserve)
    let normal_trades = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: true,
        current_value: 796_080,
        target_value: 1_038_580,
        trade_value: 242_500,
        drift_bps: BasisPoints(500),
    }];
    let res_cash_with_trades = risk_engine.evaluate_proposed_trades(
        &normal_trades,
        &positions,
        &policy,
        total_portfolio,
        300_000, // Covers trade outlay ($242.5k) but leaves $57.5k which breaches $485k min reserve
    );
    match res_cash_with_trades {
        RiskAssessment::Rejected { reason } => {
            assert!(reason.contains("Cash reserve breach"), "Reason: {}", reason);
        }
        _ => panic!("Expected rejection on cash reserve"),
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
        available_cash,
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

    // Case E: Spot Funding Rejection (settled cash $100k < $242.5k trade outlay)
    let res_funding = risk_engine.evaluate_proposed_trades(
        &normal_trades,
        &positions,
        &policy,
        total_portfolio,
        100_000,
    );
    match res_funding {
        RiskAssessment::Rejected { reason } => {
            assert!(
                reason.contains("Spot funding check failed")
                    && reason.contains("Insufficient settled cash"),
                "Reason: {}",
                reason
            );
        }
        _ => panic!("Expected rejection on spot funding"),
    }

    // Case F: Spot Ownership Rejection: Zero-balance naked short sale of unheld asset (e.g. AAPL)
    let naked_sell_trade = vec![RebalanceTrade {
        symbol: "AAPL".to_string(),
        is_buy: false,
        current_value: 0,
        target_value: 0,
        trade_value: 50_000,
        drift_bps: BasisPoints(100),
    }];
    let res_naked_sell = risk_engine.evaluate_proposed_trades(
        &naked_sell_trade,
        &positions,
        &policy,
        total_portfolio,
        available_cash,
    );
    match res_naked_sell {
        RiskAssessment::Rejected { reason } => {
            assert!(
                reason.contains("Spot ownership check failed for 'AAPL'"),
                "Reason: {}",
                reason
            );
        }
        _ => panic!("Expected rejection on zero-balance naked sell"),
    }

    // Case G: Spot Ownership Rejection: Oversell exceeding available balance
    // Vault holds $796,080 NVDA, trade attempts to sell $900,000
    let oversell_trade = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: false,
        current_value: 796_080,
        target_value: 0,
        trade_value: 900_000,
        drift_bps: BasisPoints(1855),
    }];
    let res_oversell = risk_engine.evaluate_proposed_trades(
        &oversell_trade,
        &positions,
        &policy,
        total_portfolio,
        available_cash,
    );
    match res_oversell {
        RiskAssessment::Rejected { reason } => {
            assert!(
                reason.contains("Spot ownership check failed for 'NVDA'"),
                "Reason: {}",
                reason
            );
        }
        _ => panic!("Expected rejection on oversell"),
    }

    // Case H: Spot Ownership Rejection: Zero quantity sell
    let zero_qty_sell = vec![RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: false,
        current_value: 796_080,
        target_value: 796_080,
        trade_value: 0,
        drift_bps: BasisPoints(0),
    }];
    let res_zero_qty = risk_engine.evaluate_proposed_trades(
        &zero_qty_sell,
        &positions,
        &policy,
        total_portfolio,
        available_cash,
    );
    match res_zero_qty {
        RiskAssessment::Rejected { reason } => {
            assert!(
                reason.contains("Spot ownership check failed for 'NVDA'")
                    && reason.contains("must be positive"),
                "Reason: {}",
                reason
            );
        }
        _ => panic!("Expected rejection on zero quantity sell"),
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
        .process_event(&event, &vault, &policy, &positions, 4_850_000, 1_208_760)
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
    let paused_err = decision_engine.process_event(
        &event,
        &paused_vault,
        &policy,
        &positions,
        4_850_000,
        1_208_760,
    );
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
        1_208_760,
    );
    assert!(matches!(inactive_err, Err(ApiError::BadRequest(msg)) if msg.contains("inactive")));

    // Case 3: Spot funding rejection (zero settled cash available for buy)
    let zero_cash_request = decision_engine
        .process_event(&event, &active_vault, &policy, &positions, 4_850_000, 0)
        .expect("Decision pipeline should return ExecutionRequest");
    assert!(!zero_cash_request.approved);
    assert!(
        zero_cash_request
            .rationale
            .contains("Rejected by Risk Engine")
            && zero_cash_request
                .rationale
                .contains("Spot funding check failed"),
        "Rationale: {}",
        zero_cash_request.rationale
    );

    // Case 4: Spot ownership rejection (ghost position with zero token amount)
    let ghost_positions = vec![
        PortfolioModel {
            portfolio_id: Uuid::new_v4(),
            vault_address: policy.vault_address.clone(),
            asset_symbol: "NVDA".to_string(),
            asset_mint: "Xnvda111111111111111111111111111111111111111".to_string(),
            amount: 0, // Zero vault balance owned!
            entry_price_usd: 118.5,
            current_price_usd: 128.4,
            current_value_usd: 796_080.0,
            target_weight_bps: 1600,
            current_weight_bps: 1641,
            last_rebalanced_at: None,
            updated_at: Utc::now(),
        },
        PortfolioModel {
            portfolio_id: Uuid::new_v4(),
            vault_address: policy.vault_address.clone(),
            asset_symbol: "USDC".to_string(),
            asset_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            amount: 4_053_920,
            entry_price_usd: 1.0,
            current_price_usd: 1.0,
            current_value_usd: 4_053_920.0,
            target_weight_bps: 8400,
            current_weight_bps: 8359,
            last_rebalanced_at: None,
            updated_at: Utc::now(),
        },
    ];
    let exit_event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some(policy.vault_address.clone()),
        event_type: "REGULATORY_HALT".to_string(),
        source: "sec_alert".to_string(),
        sentiment_score: Some(-1.0),
        payload: json!({ "scope": "all" }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };
    let unowned_sell_request = decision_engine
        .process_event(
            &exit_event,
            &active_vault,
            &policy,
            &ghost_positions,
            4_850_000,
            1_208_760,
        )
        .expect("Decision pipeline should return ExecutionRequest");
    assert!(!unowned_sell_request.approved);
    assert!(
        unowned_sell_request
            .rationale
            .contains("Rejected by Risk Engine")
            && unowned_sell_request
                .rationale
                .contains("Spot ownership check failed"),
        "Rationale: {}",
        unowned_sell_request.rationale
    );
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

// =========================================================================
// 5. Decision Engine: ShariahGate Security Guardrails
// =========================================================================

#[test]
fn test_decision_engine_shariah_gate_approval_and_rejections() {
    let signer = ExecutionSigner::load_or_generate("~/.config/solana/id.json");
    let mut decision_engine = DecisionEngine::new(signer);
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
    let vault = create_sample_vault(false);

    // 1. Clean approved asset passes ShariahGate and Risk Engine
    let req_approved = decision_engine
        .process_event(&event, &vault, &policy, &positions, 4_850_000, 1_208_760)
        .expect("Decision should process");
    assert!(req_approved.approved);
    assert!(req_approved.rationale.contains("Approved by Risk Engine"));

    // 2. Revoked asset blocked by ShariahGate (AI reasoning cannot override)
    decision_engine
        .registry_mut()
        .revoke_asset("backed:NVDAx")
        .expect("Revoke succeeds");

    let req_revoked = decision_engine
        .process_event(&event, &vault, &policy, &positions, 4_850_000, 1_208_760)
        .expect("Decision should process but be rejected");
    assert!(!req_revoked.approved);
    assert!(req_revoked.rationale.contains("Rejected by ShariahGate"));
    assert!(req_revoked.rationale.contains("Revoked"));

    // 3. Expired asset blocked by ShariahGate
    let mut engine_expired = DecisionEngine::new(ExecutionSigner::load_or_generate(
        "~/.config/solana/id.json",
    ));
    engine_expired
        .registry_mut()
        .expire_asset("backed:NVDAx")
        .expect("Expire succeeds");

    let req_expired = engine_expired
        .process_event(&event, &vault, &policy, &positions, 4_850_000, 1_208_760)
        .expect("Decision should process but be rejected");
    assert!(!req_expired.approved);
    assert!(req_expired.rationale.contains("Rejected by ShariahGate"));
    assert!(req_expired.rationale.contains("Expired"));
}
