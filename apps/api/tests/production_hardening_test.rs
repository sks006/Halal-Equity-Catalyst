//! Phase 18 Production Hardening Test Suite
//!
//! Validates:
//! - Secrets management and configuration validation (dev secrets blocked in mainnet)
//! - Connection string credential masking (logging security)
//! - Idempotency tracker memory pruning (leak prevention)
//! - Deterministic integer financial arithmetic (f64 prevention)
//! - Observability /metrics endpoint response
//! - Axum body size limits (DDoS / memory exhaustion prevention)

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use deadpool_postgres::{Config as DbConfig, ManagerConfig, Pool, RecyclingMethod, Runtime};
use equity_catalyst_api::{
    config::{sanitize_connection_url, Config, ConfigError},
    create_router,
    engines::{IdempotencyRecord, IdempotencyStatus, IdempotencyTracker},
    state::AppState,
};
use http_body_util::BodyExt;
use std::sync::Arc;
use tokio_postgres::NoTls;
use uuid::Uuid;

fn create_mock_pool() -> Pool {
    let mut cfg = DbConfig::new();
    cfg.url = Some("postgres://postgres:postgres@localhost:5432/mock_db".to_string());
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    cfg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap()
}

#[tokio::test]
async fn test_config_validation_rejects_dev_secret_in_mainnet() {
    let mut config = Config::mainnet();
    // 1. Missing DB url in mainnet
    assert_eq!(
        config.validate().unwrap_err(),
        ConfigError::MissingDatabaseUrl(equity_catalyst_api::config::Environment::Mainnet)
    );

    // Provide DB url
    config.database_url = "postgres://user:pass@mainnet-db.internal:5432/catalyst".to_string();

    // 2. Dev secret in mainnet
    config.admin_api_key = "catalyst-admin-secret-dev".to_string();
    let err = config.validate().unwrap_err();
    assert_eq!(
        err,
        ConfigError::DevSecretInProduction("catalyst-admin-secret-dev".to_string())
    );

    // 3. Short secret in mainnet
    config.admin_api_key = "short".to_string();
    let err2 = config.validate().unwrap_err();
    assert_eq!(
        err2,
        ConfigError::DevSecretInProduction("short".to_string())
    );

    // 4. Valid production secret passes
    config.admin_api_key = "prod-strong-entropy-secret-key-32chars!".to_string();
    config.solana_rpc_url = "https://mainnet.helius-rpc.com/?api-key=test".to_string();
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_config_validation_rejects_localhost_rpc_in_mainnet() {
    let mut config = Config::mainnet();
    config.admin_api_key = "prod-strong-entropy-secret-key-32chars!".to_string();
    config.database_url = "postgres://user:pass@mainnet-db.internal:5432/catalyst".to_string();
    config.solana_rpc_url = "http://127.0.0.1:8899".to_string();

    let err = config.validate().unwrap_err();
    assert_eq!(
        err,
        ConfigError::LocalhostRpcInProduction("http://127.0.0.1:8899".to_string())
    );
}

#[tokio::test]
async fn test_config_validation_rejects_invalid_slippage() {
    let mut config = Config::mainnet();
    config.admin_api_key = "prod-strong-entropy-secret-key-32chars!".to_string();
    config.database_url = "postgres://user:pass@mainnet-db.internal:5432/catalyst".to_string();
    config.solana_rpc_url = "https://mainnet.helius-rpc.com/?api-key=test".to_string();

    // Zero slippage rejected
    config.max_slippage_bps = 0;
    assert_eq!(config.validate().unwrap_err(), ConfigError::InvalidSlippageBps(0, 100));

    // Excessive slippage for mainnet rejected
    config.max_slippage_bps = 150;
    assert_eq!(config.validate().unwrap_err(), ConfigError::InvalidSlippageBps(150, 100));
}

#[test]
fn test_sanitize_connection_url() {
    // Redis with password
    let redis_raw = "redis://:supersecretpass@10.0.0.5:6379/0";
    let redis_clean = sanitize_connection_url(redis_raw);
    assert_eq!(redis_clean, "redis://:***@10.0.0.5:6379/0");

    // Postgres with user and password
    let pg_raw = "postgres://catalyst_admin:very_secret_pwd_99@prod-db.aws.internal:5432/equity";
    let pg_clean = sanitize_connection_url(pg_raw);
    assert_eq!(pg_clean, "postgres://catalyst_admin:***@prod-db.aws.internal:5432/equity");

    // URL without credentials remains unchanged
    let clean_raw = "redis://127.0.0.1:6379";
    assert_eq!(sanitize_connection_url(clean_raw), clean_raw);
}

#[tokio::test]
async fn test_idempotency_tracker_prune_expired() {
    let tracker = IdempotencyTracker::new();
    let current_time = 1_000_000i64;

    // Insert 1 active record and 2 expired records
    let active_id = Uuid::new_v4();
    tracker
        .check_and_register(
            IdempotencyRecord {
                idempotency_key: "idemp:active".to_string(),
                policy_decision_id: active_id,
                plan_digest: "digest-active".to_string(),
                status: IdempotencyStatus::Planned,
                created_at: current_time - 10,
                expires_at: current_time + 60, // active
            },
            current_time,
        )
        .await
        .unwrap();

    let expired_id1 = Uuid::new_v4();
    tracker
        .check_and_register(
            IdempotencyRecord {
                idempotency_key: "idemp:expired1".to_string(),
                policy_decision_id: expired_id1,
                plan_digest: "digest-exp1".to_string(),
                status: IdempotencyStatus::Completed,
                created_at: current_time - 120,
                expires_at: current_time - 30, // expired
            },
            current_time,
        )
        .await
        .unwrap();

    let expired_id2 = Uuid::new_v4();
    tracker
        .check_and_register(
            IdempotencyRecord {
                idempotency_key: "idemp:expired2".to_string(),
                policy_decision_id: expired_id2,
                plan_digest: "digest-exp2".to_string(),
                status: IdempotencyStatus::Failed,
                created_at: current_time - 200,
                expires_at: current_time - 50, // expired
            },
            current_time,
        )
        .await
        .unwrap();

    assert_eq!(tracker.len().await, 3);

    // Prune expired records
    let pruned = tracker.prune_expired(current_time).await;
    assert_eq!(pruned, 2);
    assert_eq!(tracker.len().await, 1);

    // Verify active record remains
    let active = tracker.get_record("idemp:active").await;
    assert!(active.is_some());
    assert_eq!(active.unwrap().policy_decision_id, active_id);

    // Verify expired records were removed
    assert!(tracker.get_record("idemp:expired1").await.is_none());
    assert!(tracker.get_record("idemp:expired2").await.is_none());
}

#[test]
fn test_deterministic_integer_solvency_calculation() {
    // Test cash reserve calculation without float arithmetic
    // Formula: ((total_portfolio_usd * min_cash_bps + 5000) / 10_000) as u64
    let total_portfolio_usd: u64 = 1_000_000;
    let min_cash_bps: u16 = 1_500; // 15%

    let required_cash_usd =
        ((total_portfolio_usd as u128 * min_cash_bps as u128 + 5_000) / 10_000) as u64;
    assert_eq!(required_cash_usd, 150_000);

    // Test rounding with non-even division:
    // $333,333 at 15% (1500 bps) = 49,999.95 -> rounds to 50,000
    let total_portfolio_usd2: u64 = 333_333;
    let required_cash_usd2 =
        ((total_portfolio_usd2 as u128 * min_cash_bps as u128 + 5_000) / 10_000) as u64;
    assert_eq!(required_cash_usd2, 50_000);
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let config = Config::development();
    let db_pool = create_mock_pool();
    let state = Arc::new(AppState::new(config, db_pool, None, None));
    let router = create_router(state);

    let request = Request::builder()
        .method("GET")
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    let response = tower::ServiceExt::oneshot(router, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let metrics: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(metrics["service"], "equity-catalyst-api");
    assert_eq!(metrics["environment"], "development");
    assert!(metrics["uptime_seconds"].is_number());
    assert!(metrics["rate_limit_rpm"].is_number());
    assert_eq!(metrics["read_only_mode"], false);
}

#[tokio::test]
async fn test_request_body_size_limit() {
    let config = Config::development();
    let db_pool = create_mock_pool();
    let state = Arc::new(AppState::new(config, db_pool, None, None));
    let router = create_router(state);

    // Payload exceeding 1 MB limit (e.g., 1.5 MB)
    let oversized_payload = vec![b'x'; 1_500_000];

    let request = Request::builder()
        .method("POST")
        .uri("/quotes/evaluate")
        .header("Content-Type", "application/json")
        .body(Body::from(oversized_payload))
        .unwrap();

    let response = tower::ServiceExt::oneshot(router, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
