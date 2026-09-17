use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use deadpool_postgres::Pool;
use equity_catalyst_api::{
    build_app, config::Config, create_db_pool, models::ExecutionModel,
    repositories::ExecutionRepository,
};
use http_body_util::BodyExt;
use serde_json::json;
use solana_sdk::signature::{Keypair, Signer};
use tower::ServiceExt;
use uuid::Uuid;

fn setup_test_app() -> (axum::Router, Pool) {
    let config = Config::from_env();
    let pool = create_db_pool(&config.database_url).expect("Failed to create test db pool");
    let redis_client = redis::Client::open(config.redis_url.as_str()).ok();
    let app = build_app(config, pool.clone(), redis_client);
    (app, pool)
}

#[tokio::test]
async fn test_api_get_health() {
    let (app, _) = setup_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["cluster"], "devnet");
    assert!(json["uptime_seconds"].as_u64().is_some());
}

#[tokio::test]
async fn test_api_post_and_get_vaults() {
    let (app, _) = setup_test_app();

    let vault_address = Keypair::new().pubkey().to_string();
    let authority = Keypair::new().pubkey().to_string();
    let deposit_mint = Keypair::new().pubkey().to_string();
    let token_account = Keypair::new().pubkey().to_string();

    let vault_payload = json!({
        "vault_address": vault_address,
        "authority": authority,
        "name": "API Test Vault",
        "symbol": "ATV",
        "deposit_mint": deposit_mint,
        "vault_token_account": token_account,
        "total_shares": 1000000,
        "total_deposits": 1000000,
        "is_paused": false,
        "bump": 255,
        "created_at": Utc::now().to_rfc3339(),
        "updated_at": Utc::now().to_rfc3339(),
    });

    // 1. POST /vaults
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vaults")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(vault_payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("Failed to execute POST /vaults");

    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let create_body = create_resp.into_body().collect().await.unwrap().to_bytes();
    let created_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
    assert_eq!(created_json["vault_address"], vault_address);
    assert_eq!(created_json["symbol"], "ATV");

    // 2. GET /vaults
    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/vaults")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute GET /vaults");

    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_body = list_resp.into_body().collect().await.unwrap().to_bytes();
    let list_json: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
    assert!(list_json.as_array().unwrap().len() >= 1);

    // 3. GET /vaults/:address
    let get_resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/vaults/{}", vault_address))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute GET /vaults/:address");

    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_body = get_resp.into_body().collect().await.unwrap().to_bytes();
    let get_json: serde_json::Value = serde_json::from_slice(&get_body).unwrap();
    assert_eq!(get_json["vault_address"], vault_address);
}

#[tokio::test]
async fn test_api_post_and_get_policies() {
    let (app, _) = setup_test_app();

    // First create a vault
    let vault_address = Keypair::new().pubkey().to_string();
    let authority = Keypair::new().pubkey().to_string();

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vaults")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "vault_address": vault_address,
                        "authority": authority,
                        "name": "Policy Test Vault",
                        "symbol": "PTV",
                        "deposit_mint": Keypair::new().pubkey().to_string(),
                        "vault_token_account": Keypair::new().pubkey().to_string(),
                        "total_shares": 500000,
                        "total_deposits": 500000,
                        "is_paused": false,
                        "bump": 255,
                        "created_at": Utc::now().to_rfc3339(),
                        "updated_at": Utc::now().to_rfc3339(),
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let policy_address = format!("pol_{}", Uuid::new_v4().simple());
    let policy_payload = json!({
        "policy_address": policy_address,
        "vault_address": vault_address,
        "authority": authority,
        "min_cash_bps": 1000,
        "max_position_bps": 2500,
        "stop_loss_bps": 800,
        "take_profit_bps": 2000,
        "rebalance_threshold_bps": 150,
        "is_active": true,
        "bump": 255,
        "created_at": Utc::now().to_rfc3339(),
        "updated_at": Utc::now().to_rfc3339(),
    });

    // 1. POST /policies
    let post_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/policies")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(policy_payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("Failed to execute POST /policies");

    assert_eq!(post_resp.status(), StatusCode::CREATED);
    let post_body = post_resp.into_body().collect().await.unwrap().to_bytes();
    let post_json: serde_json::Value = serde_json::from_slice(&post_body).unwrap();
    assert_eq!(post_json["policy_address"], policy_address);
    assert_eq!(post_json["vault_address"], vault_address);

    // 2. GET /policies
    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/policies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute GET /policies");

    assert_eq!(list_resp.status(), StatusCode::OK);

    // 3. GET /vaults/:address/policy
    let get_resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/vaults/{}/policy", vault_address))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute GET /vaults/:address/policy");

    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_body = get_resp.into_body().collect().await.unwrap().to_bytes();
    let get_json: serde_json::Value = serde_json::from_slice(&get_body).unwrap();
    assert_eq!(get_json["min_cash_bps"], 1000);
}

#[tokio::test]
async fn test_api_post_and_get_events() {
    let (app, _) = setup_test_app();

    // Create vault first
    let vault_address = Keypair::new().pubkey().to_string();
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vaults")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "vault_address": vault_address,
                        "authority": Keypair::new().pubkey().to_string(),
                        "name": "Event Test Vault",
                        "symbol": "ETV",
                        "deposit_mint": Keypair::new().pubkey().to_string(),
                        "vault_token_account": Keypair::new().pubkey().to_string(),
                        "total_shares": 100000,
                        "total_deposits": 100000,
                        "is_paused": false,
                        "bump": 255,
                        "created_at": Utc::now().to_rfc3339(),
                        "updated_at": Utc::now().to_rfc3339(),
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let event_id = Uuid::new_v4();
    let event_payload = json!({
        "event_id": event_id,
        "vault_address": vault_address,
        "event_type": "EARNINGS_BEAT",
        "source": "bloomberg",
        "sentiment_score": 0.85,
        "payload": { "symbol": "NVDA", "beat_pct": "+6.25%" },
        "status": "PENDING",
        "detected_at": Utc::now().to_rfc3339(),
        "processed_at": null,
    });

    // 1. POST /events
    let post_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/events")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(event_payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("Failed to execute POST /events");

    assert_eq!(post_resp.status(), StatusCode::CREATED);
    let post_body = post_resp.into_body().collect().await.unwrap().to_bytes();
    let post_json: serde_json::Value = serde_json::from_slice(&post_body).unwrap();
    assert_eq!(post_json["event_type"], "EARNINGS_BEAT");

    // 2. GET /events
    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute GET /events");

    assert_eq!(list_resp.status(), StatusCode::OK);

    // 3. GET /vaults/:address/events
    let vault_events_resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/vaults/{}/events", vault_address))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute GET /vaults/:address/events");

    assert_eq!(vault_events_resp.status(), StatusCode::OK);
    let body = vault_events_resp
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_api_get_executions() {
    let (app, pool) = setup_test_app();

    let vault_address = Keypair::new().pubkey().to_string();
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vaults")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "vault_address": vault_address,
                        "authority": Keypair::new().pubkey().to_string(),
                        "name": "Execution Test Vault",
                        "symbol": "XTV",
                        "deposit_mint": Keypair::new().pubkey().to_string(),
                        "vault_token_account": Keypair::new().pubkey().to_string(),
                        "total_shares": 100000,
                        "total_deposits": 100000,
                        "is_paused": false,
                        "bump": 255,
                        "created_at": Utc::now().to_rfc3339(),
                        "updated_at": Utc::now().to_rfc3339(),
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let exec_repo = ExecutionRepository::new(pool);
    let exec_model = ExecutionModel {
        execution_id: Uuid::new_v4(),
        vault_address: vault_address.clone(),
        event_id: None,
        action: "BUY".to_string(),
        input_mint: Keypair::new().pubkey().to_string(),
        output_mint: Keypair::new().pubkey().to_string(),
        amount_in: 50_000,
        amount_out_expected: 49_800,
        amount_out_actual: Some(49_850),
        slippage_bps: 50,
        tx_signature: Some("5xyzSignature".to_string()),
        status: "CONFIRMED".to_string(),
        error_message: None,
        executed_at: Utc::now(),
        confirmed_at: Some(Utc::now()),
    };
    exec_repo
        .create(&exec_model)
        .await
        .expect("Failed to seed execution");

    // 1. GET /executions
    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/executions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute GET /executions");

    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_body = list_resp.into_body().collect().await.unwrap().to_bytes();
    let list_json: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
    assert!(list_json.as_array().unwrap().len() >= 1);

    // 2. GET /vaults/:address/executions
    let vault_exec_resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/vaults/{}/executions", vault_address))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute GET /vaults/:address/executions");

    assert_eq!(vault_exec_resp.status(), StatusCode::OK);
    let body = vault_exec_resp
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["action"], "BUY");
}

#[tokio::test]
async fn test_api_post_dbc_configure() {
    let (app, _) = setup_test_app();

    let request_payload = json!({
        "asset": "TOKENIZED_STOCK",
        "quote_token": "USDC",
        "initial_price": 100.0,
        "curve_profile": "equity_discovery",
        "graduation_threshold": 750.0
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/dbc/configure")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&request_payload).unwrap()))
                .unwrap(),
        )
        .await
        .expect("Failed to execute POST /dbc/configure");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["asset"], "TOKENIZED_STOCK");
    assert_eq!(json["quote_token"], "USDC");
    assert_eq!(json["initial_price"], 100.0);
    assert_eq!(
        json["program_id"],
        "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"
    );
    assert_eq!(json["graduation"]["migration_option"], "MET_DAMM_V2");
    assert_eq!(json["graduation"]["migration_quote_threshold"], 750.0);

    let segments = json["segments"]
        .as_array()
        .expect("Segments must be an array");
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0]["liquidity_weight"], 1);
    assert_eq!(segments[1]["liquidity_weight"], 4);
    assert_eq!(segments[2]["liquidity_weight"], 8);
}
