//! Step 51: Health Monitor Integration Test Suite
//!
//! Validates active monitoring across the 8 critical components:
//! 1. Solana RPC
//! 2. Postgres
//! 3. Redis
//! 4. Jupiter
//! 5. Pyth
//! 6. Event Listener
//! 7. Policy Worker
//! 8. Execution Worker

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use deadpool_postgres::Pool;
use equity_catalyst_api::{
    build_app,
    config::Config,
    create_db_pool,
    services::{HealthMonitor, HealthStatus, SolanaService},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn setup_test_context() -> (
    axum::Router,
    Pool,
    Option<redis::Client>,
    Option<SolanaService>,
) {
    let config = Config::from_env();
    let pool = create_db_pool(&config.database_url).expect("Failed to connect to test Postgres");
    let redis_client = redis::Client::open(config.redis_url.as_str()).ok();
    let solana_service = Some(SolanaService::new(
        &config.solana_rpc_url,
        &config.solana_ws_url,
        None,
        None,
    ));

    let app = build_app(config, pool.clone(), redis_client.clone());
    (app, pool, redis_client, solana_service)
}

#[tokio::test]
async fn test_health_monitor_service_probes() {
    let (_, pool, redis_client, solana_service) = setup_test_context();

    let monitor = HealthMonitor::new(pool, redis_client, solana_service);
    let report = monitor.run_full_check().await;

    assert_eq!(
        report.components.len(),
        8,
        "Must probe all 8 required components"
    );

    let component_names: Vec<&str> = report.components.iter().map(|c| c.name.as_str()).collect();
    assert!(component_names.contains(&"Solana RPC"));
    assert!(component_names.contains(&"PostgreSQL"));
    assert!(component_names.contains(&"Redis"));
    assert!(component_names.contains(&"Jupiter"));
    assert!(component_names.contains(&"Pyth"));
    assert!(component_names.contains(&"event listener"));
    assert!(component_names.contains(&"policy worker"));
    assert!(component_names.contains(&"execution worker"));

    // Postgres probe must be healthy
    let pg_comp = report
        .components
        .iter()
        .find(|c| c.name == "PostgreSQL")
        .unwrap();
    assert_eq!(pg_comp.status, HealthStatus::Healthy);
    assert!(pg_comp.latency_ms.is_some());

    // Workers must report active/ready state
    let el_comp = report
        .components
        .iter()
        .find(|c| c.name == "event listener")
        .unwrap();
    assert_eq!(el_comp.status, HealthStatus::Healthy);

    let pw_comp = report
        .components
        .iter()
        .find(|c| c.name == "policy worker")
        .unwrap();
    assert_eq!(pw_comp.status, HealthStatus::Healthy);

    let ew_comp = report
        .components
        .iter()
        .find(|c| c.name == "execution worker")
        .unwrap();
    assert_eq!(ew_comp.status, HealthStatus::Healthy);
    assert_eq!(ew_comp.details["signer_isolation"], true);
}

#[tokio::test]
async fn test_health_detailed_http_endpoint() {
    let (app, _, _, _) = setup_test_context();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/detailed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute GET /health/detailed");

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(json.get("overall_status").is_some());
    assert!(json.get("uptime_seconds").is_some());

    let components = json["components"]
        .as_array()
        .expect("components must be an array");
    assert_eq!(components.len(), 8);
}
