//! Architectural validation of SOLID principles across Equity Catalyst API:
//!
//! 1. S - Single Responsibility Principle (SRP):
//!    - HTTP handlers (in `src/handlers/`) only decode requests and encode responses.
//!    - Domain logic, database access, and health checks are isolated in dedicated services and repositories.
//!
//! 2. O - Open/Closed Principle (OCP):
//!    - HealthMonitor is open to extension via `HealthCheckable` trait without modifying monitor code.
//!    - OracleService is open to custom price feeds via `PriceFeedProvider`.
//!    - QuoteExecutionService is open to custom DEX aggregators via `QuoteProvider`.
//!
//! 3. L - Liskov Substitution Principle (LSP):
//!    - `InMemoryVaultRepository`, `InMemoryPolicyRepository`, `InMemoryExecutionRepository`,
//!      `InMemoryEventRepository`, and `InMemoryDbcPoolRepository` can be substituted for PostgreSQL
//!      implementations without breaking any system contracts.
//!
//! 4. I - Interface Segregation Principle (ISP):
//!    - Narrow, role-specific traits (`VaultReader`, `VaultWriter`, `PolicyReader`, `PolicyWriter`,
//!      `ExecutionReader`, `ExecutionRecorder`) ensure consumers only depend on what they need.
//!
//! 5. D - Dependency Inversion Principle (DIP):
//!    - High-level modules (`AppState`, `handlers`, `QuoteExecutionService`, `OracleService`) depend on
//!      abstractions (traits) rather than low-level database connection pools or concrete SDK structs.

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use equity_catalyst_api::{
    build_app_in_memory,
    config::Config,
    models::{ExecutionModel, PolicyModel, VaultModel},
    repositories::{
        ExecutionRecorder, InMemoryExecutionRepository, InMemoryPolicyRepository,
        InMemoryVaultRepository, PolicyRepositoryTrait, VaultReader, VaultRepositoryTrait,
        VaultWriter,
    },
    services::{
        ComponentHealth, HealthCheckable, HealthMonitor, HealthStatus, InMemoryReadinessChecker,
        PriceFeedProvider, QuoteExecutionRequest, QuoteExecutionService, QuoteProvider,
    },
    state::AppState,
};
use equity_catalyst_jupiter::{QuoteRequest, QuoteResponse};
use equity_catalyst_pyth::{NormalizedPrice, PythError};
use http_body_util::BodyExt;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

// ============================================================================
// 1. Single Responsibility Principle (SRP) & Dependency Inversion (DIP) Tests
// ============================================================================

#[tokio::test]
async fn test_srp_and_dip_readiness_checker_isolation() {
    // Demonstrates SRP: Readiness logic is completely decoupled from HTTP handlers.
    // Demonstrates DIP: AppState depends on the ReadinessChecker trait.
    let config = Config::from_env();
    let app = build_app_in_memory(config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["ready"], true);
    assert_eq!(json["database"], "healthy");
    assert_eq!(json["redis"], "healthy");
}

#[tokio::test]
async fn test_srp_and_dip_readiness_checker_unhealthy_transition() {
    let config = Config::from_env();
    let pool = equity_catalyst_api::create_db_pool(&config.database_url).unwrap_or_else(|_| {
        let mut db_cfg = deadpool_postgres::Config::new();
        db_cfg.url = Some("postgres://dummy:dummy@localhost:5432/dummy".to_string());
        db_cfg
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .unwrap()
    });

    // Inject an unhealthy readiness checker to verify clean HTTP status mapping (503 Service Unavailable)
    let state = Arc::new(
        AppState::new_in_memory(config, pool, None, None)
            .with_readiness_checker(Arc::new(InMemoryReadinessChecker::with_unhealthy(true))),
    );
    let app = equity_catalyst_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["ready"], false);
    assert_eq!(json["database"], "unhealthy");
}

// ============================================================================
// 2. Open/Closed Principle (OCP) Tests
// ============================================================================

/// Custom mock health checker extending HealthMonitor without changing its source code.
struct CustomMicroserviceHealthCheck {
    name: String,
    healthy: bool,
}

#[async_trait]
impl HealthCheckable for CustomMicroserviceHealthCheck {
    fn component_name(&self) -> &str {
        &self.name
    }

    async fn check_health(&self) -> ComponentHealth {
        ComponentHealth {
            name: self.name.clone(),
            status: if self.healthy {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded
            },
            latency_ms: Some(15),
            details: json!({ "custom_service_connected": self.healthy }),
        }
    }
}

#[tokio::test]
async fn test_ocp_health_monitor_extension_without_modification() {
    let config = Config::from_env();
    let pool = equity_catalyst_api::create_db_pool(&config.database_url).unwrap_or_else(|_| {
        let mut db_cfg = deadpool_postgres::Config::new();
        db_cfg.url = Some("postgres://dummy:dummy@localhost:5432/dummy".to_string());
        db_cfg
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .unwrap()
    });

    let custom_checker = Arc::new(CustomMicroserviceHealthCheck {
        name: "Custom Settlement Engine".to_string(),
        healthy: true,
    });

    // HealthMonitor is OPEN for extension with new components
    let monitor = HealthMonitor::new(pool, None, None).with_checker(custom_checker);

    let report = monitor.run_full_check().await;
    let custom_component = report
        .components
        .iter()
        .find(|c| c.name == "Custom Settlement Engine");

    assert!(
        custom_component.is_some(),
        "Custom component must be evaluated"
    );
    assert_eq!(custom_component.unwrap().status, HealthStatus::Healthy);
}

/// Custom mock price feed provider extending OracleService (OCP).
struct MockPriceProvider {
    symbol: String,
    price_usd: f64,
}

#[async_trait]
impl PriceFeedProvider for MockPriceProvider {
    async fn get_normalized_price_by_symbol(
        &self,
        symbol: &str,
        _max_staleness_secs: i64,
    ) -> Result<NormalizedPrice, PythError> {
        if symbol == self.symbol {
            Ok(NormalizedPrice {
                symbol: symbol.to_string(),
                feed_id: "mock_feed".to_string(),
                price_usd: self.price_usd,
                price_scaled: (self.price_usd * 1_000_000.0) as u64,
                conf_usd: 0.01,
                expo: -8,
                publish_time: Utc::now().timestamp(),
                is_stale: false,
            })
        } else {
            Err(PythError::FeedNotFound(symbol.to_string()))
        }
    }
}

#[tokio::test]
async fn test_ocp_oracle_service_with_custom_price_provider() {
    let mock_provider = Arc::new(MockPriceProvider {
        symbol: "NVDAx".to_string(),
        price_usd: 135.50,
    });

    let oracle_service =
        equity_catalyst_api::services::OracleService::new_with_provider(mock_provider, None);

    let price = oracle_service.get_normalized_price("NVDAx").await.unwrap();
    assert_eq!(price.price_usd, 135.50);

    let not_found = oracle_service.get_normalized_price("UNKNOWN").await;
    assert!(not_found.is_err());
}

/// Custom mock quote provider extending QuoteExecutionService (OCP).
struct MockQuoteProvider;

#[async_trait]
impl QuoteProvider for MockQuoteProvider {
    async fn get_quote(
        &self,
        request: &QuoteRequest,
    ) -> Result<QuoteResponse, equity_catalyst_jupiter::JupiterError> {
        Ok(QuoteResponse {
            input_mint: request.input_mint.clone(),
            in_amount: request.amount.to_string(),
            output_mint: request.output_mint.clone(),
            out_amount: "50000000".to_string(),
            other_amount_threshold: "49500000".to_string(),
            swap_mode: "ExactIn".to_string(),
            slippage_bps: request.slippage_bps.unwrap_or(50),
            price_impact_pct: "0.01".to_string(),
            route_plan: vec![],
            context_slot: None,
            time_taken: None,
        })
    }
}

#[tokio::test]
async fn test_ocp_quote_execution_service_with_custom_provider() {
    let mock_quote_provider = Arc::new(MockQuoteProvider);
    let risk_engine = Arc::new(equity_catalyst_api::engines::risk_engine::RiskEngine::new());

    let quote_service = QuoteExecutionService::new_with_traits(
        mock_quote_provider,
        risk_engine,
        None,
        None,
        None,
        None,
    );

    let request = QuoteExecutionRequest {
        vault_address: "Vault1111111111111111111111111111111111111".to_string(),
        input_mint: "MintA1111111111111111111111111111111111111".to_string(),
        output_mint: "MintB1111111111111111111111111111111111111".to_string(),
        amount_in: 1_000_000,
        slippage_bps: Some(50),
        target_symbol: None,
    };

    let verdict = quote_service.evaluate_quote(&request).await.unwrap();
    assert!(verdict.approved);
    assert_eq!(verdict.expected_amount_out, 50_000_000);
}

// ============================================================================
// 3. Liskov Substitution Principle (LSP) Tests
// ============================================================================

#[tokio::test]
async fn test_lsp_in_memory_repository_contract_substitutability() {
    // Any consumer expecting `Arc<dyn VaultRepositoryTrait>` works seamlessly
    // with the in-memory implementation without breaking behavioral expectations.
    let repo: Arc<dyn VaultRepositoryTrait> = Arc::new(InMemoryVaultRepository::new());

    let vault = VaultModel {
        vault_address: "V11111111111111111111111111111111111111111".to_string(),
        authority: "Auth11111111111111111111111111111111111111".to_string(),
        name: "LSP Test Vault".to_string(),
        symbol: "LSP".to_string(),
        deposit_mint: "Mint11111111111111111111111111111111111111".to_string(),
        vault_token_account: "Acc11111111111111111111111111111111111111".to_string(),
        total_shares: 50_000,
        total_deposits: 50_000,
        is_paused: false,
        bump: 254,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // 1. Create vault
    let created = repo.create(&vault).await.unwrap();
    assert_eq!(created.vault_address, vault.vault_address);

    // 2. Query vault
    let fetched = repo.find_by_address(&vault.vault_address).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "LSP Test Vault");

    // 3. Mutate totals
    repo.update_totals(&vault.vault_address, 100_000, 100_000)
        .await
        .unwrap();
    let updated = repo
        .find_by_address(&vault.vault_address)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.total_shares, 100_000);

    // 4. Toggle pause
    repo.set_paused(&vault.vault_address, true).await.unwrap();
    let paused = repo
        .find_by_address(&vault.vault_address)
        .await
        .unwrap()
        .unwrap();
    assert!(paused.is_paused);

    // 5. Query non-existent
    let non_existent = repo.find_by_address("NonExistentAddress").await.unwrap();
    assert!(non_existent.is_none());
}

#[tokio::test]
async fn test_lsp_policy_and_execution_in_memory_substitutability() {
    let policy_repo: Arc<dyn PolicyRepositoryTrait> = Arc::new(InMemoryPolicyRepository::new());
    let exec_repo: Arc<dyn ExecutionRecorder> = Arc::new(InMemoryExecutionRepository::new());

    let policy = PolicyModel {
        policy_address: "Pol11111111111111111111111111111111111111".to_string(),
        vault_address: "Vault111111111111111111111111111111111111".to_string(),
        authority: "Auth1111111111111111111111111111111111111".to_string(),
        min_cash_bps: 1000,
        max_position_bps: 3000,
        stop_loss_bps: 500,
        take_profit_bps: 2000,
        rebalance_threshold_bps: 100,
        is_active: true,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let saved = policy_repo.upsert(&policy).await.unwrap();
    assert_eq!(saved.policy_address, policy.policy_address);

    let fetched = policy_repo
        .find_by_vault(&policy.vault_address)
        .await
        .unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().max_position_bps, 3000);

    let execution = ExecutionModel {
        execution_id: Uuid::new_v4(),
        vault_address: policy.vault_address.clone(),
        event_id: None,
        action: "TEST_SWAP".to_string(),
        input_mint: "A".to_string(),
        output_mint: "B".to_string(),
        amount_in: 500,
        amount_out_expected: 500,
        amount_out_actual: Some(499),
        slippage_bps: 50,
        tx_signature: Some("sig123".to_string()),
        status: "CONFIRMED".to_string(),
        error_message: None,
        executed_at: Utc::now(),
        confirmed_at: Some(Utc::now()),
        quote_id: None,
        policy_decision_id: None,
        amount_out_min: None,
    };

    let recorded = exec_repo.create(&execution).await.unwrap();
    assert_eq!(recorded.execution_id, execution.execution_id);
}

// ============================================================================
// 4. Interface Segregation Principle (ISP) Tests
// ============================================================================

/// Function that strictly requires `VaultReader` and has no access to mutation methods.
async fn query_vault_name(reader: &dyn VaultReader, address: &str) -> Option<String> {
    reader
        .find_by_address(address)
        .await
        .unwrap()
        .map(|v| v.name)
}

/// Function that strictly requires `ExecutionRecorder` and cannot query execution history.
async fn record_audit_trail(recorder: &dyn ExecutionRecorder, exec: &ExecutionModel) -> Uuid {
    recorder.create(exec).await.unwrap().execution_id
}

#[tokio::test]
async fn test_isp_segregated_reader_and_writer_interfaces() {
    let in_memory_vaults = InMemoryVaultRepository::new();

    let vault = VaultModel {
        vault_address: "SegregatedVault123".to_string(),
        authority: "Auth123".to_string(),
        name: "ISP Segregated Vault".to_string(),
        symbol: "ISPV".to_string(),
        deposit_mint: "Mint123".to_string(),
        vault_token_account: "Acc123".to_string(),
        total_shares: 1000,
        total_deposits: 1000,
        is_paused: false,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Use VaultWriter to insert
    in_memory_vaults.create(&vault).await.unwrap();

    // Pass ONLY the VaultReader trait slice to the consumer
    let reader_ref: &dyn VaultReader = &in_memory_vaults;
    let name = query_vault_name(reader_ref, "SegregatedVault123").await;
    assert_eq!(name, Some("ISP Segregated Vault".to_string()));

    // Pass ONLY the ExecutionRecorder to the auditor
    let in_memory_exec = InMemoryExecutionRepository::new();
    let recorder_ref: &dyn ExecutionRecorder = &in_memory_exec;

    let exec = ExecutionModel {
        execution_id: Uuid::new_v4(),
        vault_address: "SegregatedVault123".to_string(),
        event_id: None,
        action: "ISP_AUDIT".to_string(),
        input_mint: "In".to_string(),
        output_mint: "Out".to_string(),
        amount_in: 100,
        amount_out_expected: 100,
        amount_out_actual: None,
        slippage_bps: 10,
        tx_signature: None,
        status: "PENDING".to_string(),
        error_message: None,
        executed_at: Utc::now(),
        confirmed_at: None,
        quote_id: None,
        policy_decision_id: None,
        amount_out_min: None,
    };

    let id = record_audit_trail(recorder_ref, &exec).await;
    assert_eq!(id, exec.execution_id);
}

// ============================================================================
// 5. End-to-End API Router Test using In-Memory Dependency Container (DIP)
// ============================================================================

#[tokio::test]
async fn test_end_to_end_api_with_in_memory_container() {
    let mut config = Config::from_env();
    config.admin_api_key = "admin-secret-token".to_string();
    let app = build_app_in_memory(config);

    // 1. Create a vault via POST /vaults
    let create_vault_req = json!({
        "vault_address": "TestVault11111111111111111111111111111111",
        "authority": "Authority11111111111111111111111111111111",
        "name": "SOLID Test Vault",
        "symbol": "STV",
        "deposit_mint": "Mint11111111111111111111111111111111111111",
        "vault_token_account": "Account11111111111111111111111111111111111",
        "total_shares": 500000,
        "total_deposits": 500000,
        "is_paused": false,
        "bump": 255,
        "created_at": Utc::now().to_rfc3339(),
        "updated_at": Utc::now().to_rfc3339(),
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vaults")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer admin-secret-token")
                .body(Body::from(serde_json::to_vec(&create_vault_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);

    // 2. Fetch the created vault via GET /vaults/:address
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/vaults/TestVault11111111111111111111111111111111")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "SOLID Test Vault");
    assert_eq!(json["symbol"], "STV");

    // 3. Create a policy via POST /policies
    let policy_payload = json!({
        "policy_address": "Policy111111111111111111111111111111111111",
        "vault_address": "TestVault11111111111111111111111111111111",
        "authority": "Authority11111111111111111111111111111111",
        "min_cash_bps": 1000,
        "max_position_bps": 2500,
        "stop_loss_bps": 500,
        "take_profit_bps": 1500,
        "rebalance_threshold_bps": 100,
        "is_active": true,
        "bump": 255,
        "created_at": Utc::now().to_rfc3339(),
        "updated_at": Utc::now().to_rfc3339(),
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/policies")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer admin-secret-token")
                .body(Body::from(serde_json::to_vec(&policy_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);

    // 4. Fetch policy via GET /policies/:vault_address
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/policies/TestVault11111111111111111111111111111111")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["max_position_bps"], 2500);

    // 5. Test metrics endpoint GET /metrics
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
