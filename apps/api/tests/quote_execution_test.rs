use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use deadpool_postgres::Pool;
use equity_catalyst_api::{
    config::Config,
    create_db_pool,
    engines::risk_engine::RiskEngine,
    models::{PolicyModel, VaultModel},
    repositories::{ExecutionRepository, PolicyRepository, PortfolioRepository, VaultRepository},
    router::create_router,
    services::{QuoteExecutionRequest, QuoteExecutionService, QuoteExecutionVerdict},
    state::AppState,
};
use equity_catalyst_jupiter::{JupiterClient, QuoteResponse, RoutePlanStep, SwapInfo};
use http_body_util::BodyExt;
use std::sync::Arc;
use uuid::Uuid;

fn test_config() -> Config {
    Config {
        database_url: "postgres://postgres:postgres@localhost:5432/equity_catalyst".to_string(),
        redis_url: "redis://127.0.0.1:6379".to_string(),
        ..Config::default()
    }
}

fn get_db_pool() -> Pool {
    create_db_pool(&test_config().database_url).expect("Failed to connect to test Postgres")
}

fn create_mock_quote(
    input_mint: &str,
    output_mint: &str,
    in_amount: &str,
    out_amount: &str,
    price_impact_pct: &str,
) -> QuoteResponse {
    QuoteResponse {
        input_mint: input_mint.to_string(),
        in_amount: in_amount.to_string(),
        output_mint: output_mint.to_string(),
        out_amount: out_amount.to_string(),
        other_amount_threshold: out_amount.to_string(),
        swap_mode: "ExactIn".to_string(),
        slippage_bps: 50,
        price_impact_pct: price_impact_pct.to_string(),
        route_plan: vec![RoutePlanStep {
            swap_info: SwapInfo {
                amm_key: "Whirlpool111111111111111111111111111111111".to_string(),
                label: Some("Orca Whirlpool".to_string()),
                input_mint: input_mint.to_string(),
                output_mint: output_mint.to_string(),
                in_amount: in_amount.to_string(),
                out_amount: out_amount.to_string(),
                fee_amount: Some("100".to_string()),
                fee_mint: Some(input_mint.to_string()),
            },
            percent: 100,
        }],
        context_slot: Some(300_000_000),
        time_taken: Some(0.015),
    }
}

#[tokio::test]
async fn test_step33_quote_only_execution_approval_flow() {
    let pool = get_db_pool();
    let vault_repo = VaultRepository::new(pool.clone());
    let policy_repo = PolicyRepository::new(pool.clone());
    let port_repo = PortfolioRepository::new(pool.clone());
    let exec_repo = ExecutionRepository::new(pool.clone());

    let vault_addr = format!("QuoteVault_{}", Uuid::new_v4().simple());
    let sol_mint = "So11111111111111111111111111111111111111112";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    // 1. Seed vault
    let vault = VaultModel {
        vault_address: vault_addr.clone(),
        authority: "Authority_QuoteTest".to_string(),
        name: "Quote Test Vault".to_string(),
        symbol: "QTV".to_string(),
        deposit_mint: usdc_mint.to_string(),
        vault_token_account: "Token_QuoteAcct".to_string(),
        total_shares: 500_000,
        total_deposits: 500_000,
        is_paused: false,
        bump: 254,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    vault_repo
        .create(&vault)
        .await
        .expect("Vault creation failed");

    // 2. Seed active policy: 3% max rebalance drift (300 bps), 30% max position (3000 bps)
    let policy = PolicyModel {
        policy_address: format!("Policy_{}", Uuid::new_v4().simple()),
        vault_address: vault_addr.clone(),
        authority: "Authority_QuoteTest".to_string(),
        max_ltv_bps: 7_500,
        max_position_bps: 3_000,
        stop_loss_bps: 500,
        take_profit_bps: 1_500,
        rebalance_threshold_bps: 300,
        is_active: true,
        bump: 253,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    policy_repo
        .upsert(&policy)
        .await
        .expect("Policy upsert failed");

    // 3. Configure Jupiter mock client with 0.05% (5 bps) price impact
    let jupiter_mock = Arc::new(JupiterClient::new_mock());
    let quote = create_mock_quote(sol_mint, usdc_mint, "1000000000", "145000000", "0.05");
    jupiter_mock.set_mock_quote(sol_mint, usdc_mint, quote);

    let risk_engine = Arc::new(RiskEngine::new());
    let quote_service = QuoteExecutionService::new(
        jupiter_mock,
        risk_engine,
        Some(vault_repo),
        Some(policy_repo),
        Some(port_repo),
        Some(exec_repo.clone()),
    );

    // 4. Request quote evaluation
    let req = QuoteExecutionRequest {
        vault_address: vault_addr.clone(),
        input_mint: sol_mint.to_string(),
        output_mint: usdc_mint.to_string(),
        amount_in: 1_000_000_000, // 1 SOL
        slippage_bps: Some(50),
        target_symbol: Some("USDC".to_string()),
    };

    let verdict = quote_service
        .evaluate_quote(&req)
        .await
        .expect("Quote evaluation failed");

    // 5. Verify outcomes
    assert!(verdict.approved, "Quote should be approved by Risk Engine");
    assert_eq!(verdict.expected_amount_out, 145_000_000);
    assert_eq!(verdict.price_impact_bps, 5);
    assert!(
        verdict.is_dry_run,
        "Dry-run flag must be true (no real swap executed)"
    );
    assert!(verdict.rejection_reason.is_none());

    // 6. Verify audit entry in PostgreSQL
    let saved_exec = exec_repo
        .find_by_id(verdict.execution_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_exec.status, "QUOTE_ONLY");
    assert!(
        saved_exec.tx_signature.is_none(),
        "No on-chain transaction signature allowed in quote mode"
    );
    assert_eq!(saved_exec.amount_out_expected, 145_000_000);
}

#[tokio::test]
async fn test_step33_quote_rejection_on_excessive_price_impact() {
    let pool = get_db_pool();
    let vault_repo = VaultRepository::new(pool.clone());
    let policy_repo = PolicyRepository::new(pool.clone());
    let port_repo = PortfolioRepository::new(pool.clone());
    let exec_repo = ExecutionRepository::new(pool.clone());

    let vault_addr = format!("ImpactVault_{}", Uuid::new_v4().simple());
    let sol_mint = "So11111111111111111111111111111111111111112";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    let vault = VaultModel {
        vault_address: vault_addr.clone(),
        authority: "Authority_ImpactTest".to_string(),
        name: "Impact Test Vault".to_string(),
        symbol: "ITV".to_string(),
        deposit_mint: usdc_mint.to_string(),
        vault_token_account: "Token_ImpactAcct".to_string(),
        total_shares: 100_000,
        total_deposits: 100_000,
        is_paused: false,
        bump: 254,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    vault_repo
        .create(&vault)
        .await
        .expect("Vault creation failed");

    // Policy allows up to 2.00% (200 bps) impact
    let policy = PolicyModel {
        policy_address: format!("Policy_{}", Uuid::new_v4().simple()),
        vault_address: vault_addr.clone(),
        authority: "Authority_ImpactTest".to_string(),
        max_ltv_bps: 7_500,
        max_position_bps: 2_500,
        stop_loss_bps: 500,
        take_profit_bps: 1_500,
        rebalance_threshold_bps: 200,
        is_active: true,
        bump: 253,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    policy_repo
        .upsert(&policy)
        .await
        .expect("Policy upsert failed");

    // Mock quote with 4.50% (450 bps) price impact
    let jupiter_mock = Arc::new(JupiterClient::new_mock());
    let quote = create_mock_quote(sol_mint, usdc_mint, "10000000000", "1300000000", "4.50");
    jupiter_mock.set_mock_quote(sol_mint, usdc_mint, quote);

    let risk_engine = Arc::new(RiskEngine::new());
    let quote_service = QuoteExecutionService::new(
        jupiter_mock,
        risk_engine,
        Some(vault_repo),
        Some(policy_repo),
        Some(port_repo),
        Some(exec_repo.clone()),
    );

    let req = QuoteExecutionRequest {
        vault_address: vault_addr.clone(),
        input_mint: sol_mint.to_string(),
        output_mint: usdc_mint.to_string(),
        amount_in: 10_000_000_000,
        slippage_bps: Some(50),
        target_symbol: Some("USDC".to_string()),
    };

    let verdict = quote_service
        .evaluate_quote(&req)
        .await
        .expect("Evaluation should succeed with rejection verdict");

    assert!(
        !verdict.approved,
        "Quote should be rejected due to excessive price impact"
    );
    assert_eq!(verdict.price_impact_bps, 450);
    assert!(verdict.rejection_reason.is_some());
    assert!(verdict
        .rejection_reason
        .unwrap()
        .contains("breaches maximum allowed limit"));

    // Check DB status is REJECTED
    let saved_exec = exec_repo
        .find_by_id(verdict.execution_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_exec.status, "REJECTED");
}

#[tokio::test]
async fn test_step33_quote_rejection_on_paused_vault() {
    let pool = get_db_pool();
    let vault_repo = VaultRepository::new(pool.clone());
    let policy_repo = PolicyRepository::new(pool.clone());

    let vault_addr = format!("PausedVault_{}", Uuid::new_v4().simple());
    let sol_mint = "So11111111111111111111111111111111111111112";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    let vault = VaultModel {
        vault_address: vault_addr.clone(),
        authority: "Authority_Paused".to_string(),
        name: "Paused Vault".to_string(),
        symbol: "PV".to_string(),
        deposit_mint: usdc_mint.to_string(),
        vault_token_account: "Token_PausedAcct".to_string(),
        total_shares: 100_000,
        total_deposits: 100_000,
        is_paused: true, // PAUSED
        bump: 254,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    vault_repo
        .create(&vault)
        .await
        .expect("Vault creation failed");

    let policy = PolicyModel {
        policy_address: format!("Policy_{}", Uuid::new_v4().simple()),
        vault_address: vault_addr.clone(),
        authority: "Authority_Paused".to_string(),
        max_ltv_bps: 7_500,
        max_position_bps: 2_500,
        stop_loss_bps: 500,
        take_profit_bps: 1_500,
        rebalance_threshold_bps: 200,
        is_active: true,
        bump: 253,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    policy_repo
        .upsert(&policy)
        .await
        .expect("Policy upsert failed");

    let jupiter_mock = Arc::new(JupiterClient::new_mock());
    let quote = create_mock_quote(sol_mint, usdc_mint, "1000000", "145000", "0.01");
    jupiter_mock.set_mock_quote(sol_mint, usdc_mint, quote);

    let quote_service = QuoteExecutionService::new(
        jupiter_mock,
        Arc::new(RiskEngine::new()),
        Some(vault_repo),
        Some(policy_repo),
        None,
        None,
    );

    let req = QuoteExecutionRequest {
        vault_address: vault_addr.clone(),
        input_mint: sol_mint.to_string(),
        output_mint: usdc_mint.to_string(),
        amount_in: 1_000_000,
        slippage_bps: Some(50),
        target_symbol: None,
    };

    let verdict = quote_service.evaluate_quote(&req).await.unwrap();
    assert!(!verdict.approved);
    assert!(verdict.rejection_reason.unwrap().contains("paused"));
}

#[tokio::test]
async fn test_quote_evaluate_http_endpoint() {
    let pool = get_db_pool();
    let sol_mint = "So11111111111111111111111111111111111111112";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    let jupiter_mock = Arc::new(JupiterClient::new_mock());
    let quote = create_mock_quote(sol_mint, usdc_mint, "2000000000", "290000000", "0.02");
    jupiter_mock.set_mock_quote(sol_mint, usdc_mint, quote);

    let quote_service = Arc::new(QuoteExecutionService::new(
        jupiter_mock,
        Arc::new(RiskEngine::new()),
        None,
        None,
        None,
        None,
    ));

    let state =
        Arc::new(AppState::new(test_config(), pool, None, None).with_quote_service(quote_service));
    let app = create_router(state);

    let request_payload = serde_json::json!({
        "vault_address": "EndpointVault_123",
        "input_mint": sol_mint,
        "output_mint": usdc_mint,
        "amount_in": 2000000000u64,
        "slippage_bps": 50
    });

    let request = Request::builder()
        .uri("/quotes/evaluate")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&request_payload).unwrap()))
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let verdict: QuoteExecutionVerdict = serde_json::from_slice(&body).unwrap();
    assert!(verdict.approved);
    assert_eq!(verdict.expected_amount_out, 290_000_000);
    assert_eq!(verdict.price_impact_bps, 2);
    assert!(verdict.is_dry_run);
}
