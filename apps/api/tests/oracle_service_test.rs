use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use deadpool_postgres::Pool;
use equity_catalyst_api::{
    config::Config,
    create_db_pool,
    models::{PortfolioModel, VaultModel},
    repositories::{PortfolioRepository, VaultRepository},
    router::create_router,
    services::OracleService,
    state::AppState,
};
use equity_catalyst_pyth::PythClient;
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

#[tokio::test]
async fn test_oracle_service_price_normalization_and_confidence() {
    let pyth_mock = Arc::new(PythClient::new_mock());
    let now = Utc::now().timestamp();

    // Mock SOL at $145.50
    pyth_mock.set_mock_price("SOL", "14550000000", "20000000", -8, now);

    let oracle = OracleService::new(pyth_mock.clone(), None)
        .with_staleness_limit(60)
        .with_confidence_limit(0.02); // 2% max confidence

    let price = oracle
        .get_normalized_price("SOL")
        .await
        .expect("Failed to get normalized SOL price");

    assert_eq!(price.symbol, "SOL");
    assert!((price.price_usd - 145.50).abs() < 1e-4);
    assert_eq!(price.price_scaled, 145_500_000);
    assert!(!price.is_stale);

    // Test rejection on excessive confidence width (> 2%)
    pyth_mock.set_mock_price("SOL", "14550000000", "500000000", -8, now); // ~3.4% conf
    let result = oracle.get_normalized_price("SOL").await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("confidence interval too wide"));
}

#[tokio::test]
async fn test_oracle_service_portfolio_valuation_sync() {
    let pool = get_db_pool();
    let vault_repo = VaultRepository::new(pool.clone());
    let port_repo = PortfolioRepository::new(pool.clone());

    let pyth_mock = Arc::new(PythClient::new_mock());
    let now = Utc::now().timestamp();

    // Set mock prices: SOL = $150.00, AAPL = $220.00
    pyth_mock.set_mock_price("SOL", "15000000000", "10000000", -8, now);
    pyth_mock.set_mock_price("AAPL", "22000000000", "10000000", -8, now);

    let oracle = OracleService::new(pyth_mock, Some(port_repo.clone()));

    // 1. Seed vault
    let vault_addr = format!("OracleVault_{}", Uuid::new_v4().simple());
    let vault = VaultModel {
        vault_address: vault_addr.clone(),
        authority: "Authority_OracleTest".to_string(),
        name: "Oracle Sync Vault".to_string(),
        symbol: "OSV".to_string(),
        deposit_mint: "USDC_OracleMint".to_string(),
        vault_token_account: "Token_OracleAcct".to_string(),
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
        .expect("Vault create failed");

    // 2. Seed 2 positions with initial outdated prices
    let pos_sol = PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: vault_addr.clone(),
        asset_symbol: "SOL".to_string(),
        asset_mint: "So11111111111111111111111111111111111111112".to_string(),
        amount: 100, // 100 units
        entry_price_usd: 120.0,
        current_price_usd: 120.0,
        current_value_usd: 12_000.0, // 100 * 120
        target_weight_bps: 5_000,
        current_weight_bps: 5_000,
        last_rebalanced_at: None,
        updated_at: Utc::now(),
    };
    port_repo
        .upsert_position(&pos_sol)
        .await
        .expect("Failed to seed SOL position");

    let pos_aapl = PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: vault_addr.clone(),
        asset_symbol: "AAPL".to_string(),
        asset_mint: "AAPL_Mint111111111111111111111111111111111".to_string(),
        amount: 50, // 50 units
        entry_price_usd: 200.0,
        current_price_usd: 200.0,
        current_value_usd: 10_000.0, // 50 * 200
        target_weight_bps: 5_000,
        current_weight_bps: 5_000,
        last_rebalanced_at: None,
        updated_at: Utc::now(),
    };
    port_repo
        .upsert_position(&pos_aapl)
        .await
        .expect("Failed to seed AAPL position");

    // 3. Trigger oracle portfolio valuation sync
    let updated = oracle
        .update_portfolio_valuation(&vault_addr)
        .await
        .expect("Valuation sync failed");

    assert_eq!(updated.len(), 2);

    // SOL should now be $150.00 -> 100 * 150 = $15,000
    let sol_updated = updated.iter().find(|p| p.asset_symbol == "SOL").unwrap();
    assert!((sol_updated.current_price_usd - 150.0).abs() < 1e-4);
    assert!((sol_updated.current_value_usd - 15_000.0).abs() < 1e-4);

    // AAPL should now be $220.00 -> 50 * 220 = $11,000
    let aapl_updated = updated.iter().find(|p| p.asset_symbol == "AAPL").unwrap();
    assert!((aapl_updated.current_price_usd - 220.0).abs() < 1e-4);
    assert!((aapl_updated.current_value_usd - 11_000.0).abs() < 1e-4);

    // Total portfolio value = 15,000 + 11,000 = 26,000
    // SOL weight = 15,000 / 26,000 = 57.69% -> ~5769 bps
    // AAPL weight = 11,000 / 26,000 = 42.31% -> ~4231 bps
    assert_eq!(sol_updated.current_weight_bps, 5769);
    assert_eq!(aapl_updated.current_weight_bps, 4231);
}

#[tokio::test]
async fn test_oracle_route_http_endpoint() {
    let pool = get_db_pool();
    let pyth_mock = Arc::new(PythClient::new_mock());
    let now = Utc::now().timestamp();
    pyth_mock.set_mock_price("BTC", "6500000000000", "500000000", -8, now);

    let oracle_service = Arc::new(OracleService::new(pyth_mock, None));

    let state = Arc::new(
        AppState::new(test_config(), pool, None, None).with_oracle_service(oracle_service),
    );
    let app = create_router(state);

    let request = Request::builder()
        .uri("/oracle/price/BTC")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("\"symbol\":\"BTC\""));
    assert!(body_str.contains("\"price_usd\":65000"));
}
