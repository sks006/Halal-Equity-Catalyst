//! Integration and Recovery Test Suite for Trading Pipeline Safety (Phase 16).
//!
//! # Objective
//! Make the trading pipeline fail closed under critical data and infrastructure failures.
//!
//! # Principle
//! **Critical failures must prevent execution rather than silently continuing with unsafe state.**
//!
//! # Verified 15 Failure Scenarios & Recovery:
//! 1. Pyth unavailable
//! 2. Pyth stream disconnected
//! 3. Pyth data stale
//! 4. Pyth confidence too large
//! 5. Wrong feed
//! 6. Asset deactivated
//! 7. Shariah approval revoked
//! 8. DEX quote expired
//! 9. DEX unavailable
//! 10. Signer unavailable
//! 11. Transaction submission failure
//! 12. Transaction confirmation timeout
//! 13. Duplicate execution request
//! 14. Database unavailable
//! 15. WebSocket clients disconnected

use equity_catalyst_api::engines::pipeline_safety::{
    PipelineExecutionContext, PipelineState, TradingPipelineSafetyGuard,
};
use equity_catalyst_api::services::MarketPriceUpdate;
use equity_catalyst_shared::shariah::ShariahStatus;
use uuid::Uuid;

/// Helper function to create a pristine, valid execution context where all 15 invariants pass.
fn create_baseline_valid_context<'a>(
    symbol: &'a str,
    feed_id: &'a str,
    price_update: &'a Result<MarketPriceUpdate, String>,
    dex_result: &'a Result<(), String>,
    sub_result: &'a Result<String, String>,
    db_result: &'a Result<(), String>,
    execution_id: Uuid,
    now: i64,
) -> PipelineExecutionContext<'a> {
    PipelineExecutionContext {
        symbol,
        expected_feed_id: feed_id,
        price_update,
        is_stream_connected: true,
        is_asset_active: true,
        asset_status: "Active",
        shariah_status: ShariahStatus::Approved,
        shariah_reviewed_at: now - 3600,
        shariah_expires_at: now + 86400 * 90,
        quote_id: "dex-quote-1001",
        quote_expires_at: now + 30, // 30s remaining TTL
        dex_result,
        is_signer_available: true,
        signer_type: "RemoteHsmSigner",
        submission_result: sub_result,
        is_confirmed: true,
        confirmation_timeout_secs: 30,
        execution_id,
        is_duplicate: false,
        existing_status: None,
        db_result,
        is_ws_connected: true,
        now,
    }
}

fn create_valid_price_update(symbol: &str, feed_id: &str, now: i64) -> MarketPriceUpdate {
    MarketPriceUpdate::from_raw(
        format!("backed:{}", symbol),
        symbol,
        "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
        feed_id,
        12_500_000_000, // $125.00
        -8,
        25_000_000, // 20 bps confidence (0.20% <= 50 bps limit)
        now - 5,    // 5 seconds old (fresh <= 30s limit)
        30,
        Some(100_000),
    )
    .expect("Valid price update")
}

// =============================================================================
// 1. PYTH UNAVAILABLE: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_01_pyth_unavailable_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();

    // 1. Failure: Pyth Hermes API is unreachable (HTTP 503 Service Unavailable)
    let err_price: Result<MarketPriceUpdate, String> =
        Err("HTTP 503 Service Unavailable: Pyth Hermes cluster timeout".to_string());
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &err_price,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(result.is_err(), "Must fail closed when Pyth is unavailable");
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "PYTH_UNAVAILABLE");
    assert_eq!(failure.http_status_code(), 503);
    assert!(failure.is_transient());

    // 2. Recovery: Pyth Hermes API recovers and serves price
    let fresh_price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    ctx.price_update = &fresh_price;

    let recovery_result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        recovery_result.is_ok(),
        "Must recover once Pyth becomes available"
    );
    assert_eq!(recovery_result.unwrap(), PipelineState::Completed);
}

// =============================================================================
// 2. PYTH STREAM DISCONNECTED: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_02_pyth_stream_disconnected_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    // 1. Failure: Real-time SSE price stream disconnected
    ctx.is_stream_connected = false;
    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_err(),
        "Must fail closed when Pyth stream is disconnected"
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "PYTH_STREAM_DISCONNECTED");
    assert_eq!(failure.http_status_code(), 503);

    // 2. Recovery: SSE stream reconnects successfully
    ctx.is_stream_connected = true;
    let recovery_result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        recovery_result.is_ok(),
        "Must recover once Pyth stream reconnects"
    );
}

// =============================================================================
// 3. PYTH DATA STALE: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_03_pyth_data_stale_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    // 1. Failure: Stale price (60 seconds old, threshold is 30s)
    let stale_price = MarketPriceUpdate::from_raw(
        "backed:NVDA",
        "NVDA",
        "Mint111111111111111111111111111111111111111",
        "0xfeed1234",
        12_000_000_000,
        -8,
        500_000,
        now - 60, // 60s stale
        30,
        None,
    )
    .unwrap();
    let stale_res: Result<MarketPriceUpdate, String> = Ok(stale_price);

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &stale_res,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(result.is_err(), "Must fail closed when Pyth data is stale");
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "PYTH_DATA_STALE");
    assert_eq!(failure.http_status_code(), 400);

    // 2. Recovery: Fresh price update published 2 seconds ago
    let fresh_price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    ctx.price_update = &fresh_price;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 4. PYTH CONFIDENCE TOO LARGE: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_04_pyth_confidence_too_large_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    // 1. Failure: Confidence interval 150 bps (1.50% > 50 bps limit)
    let wide_conf_price = MarketPriceUpdate::from_raw(
        "backed:NVDA",
        "NVDA",
        "Mint111111111111111111111111111111111111111",
        "0xfeed1234",
        10_000_000_000, // $100.00
        -8,
        150_000_000, // 150 bps confidence width
        now - 2,
        30,
        None,
    )
    .unwrap();
    let wide_res: Result<MarketPriceUpdate, String> = Ok(wide_conf_price);

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &wide_res,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_err(),
        "Must fail closed when Pyth confidence is too wide"
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "PYTH_CONFIDENCE_TOO_LARGE");

    // 2. Recovery: Confidence narrows to 10 bps
    let tight_price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    ctx.price_update = &tight_price;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 5. WRONG FEED: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_05_wrong_feed_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    // 1. Failure: Price update arrives with wrong feed ID (e.g. BTC feed for NVDA asset)
    let wrong_feed_price = MarketPriceUpdate::from_raw(
        "backed:NVDA",
        "NVDA",
        "Mint111111111111111111111111111111111111111",
        "0xdeadbeef_wrong_feed",
        10_000_000_000,
        -8,
        10_000_000,
        now - 2,
        30,
        None,
    )
    .unwrap();
    let wrong_res: Result<MarketPriceUpdate, String> = Ok(wrong_feed_price);

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234", // Expected feed
        &wrong_res,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_err(),
        "Must fail closed when feed ID does not match"
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "WRONG_FEED");

    // 2. Recovery: Price update with matching canonical feed ID arrives
    let correct_feed_price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    ctx.price_update = &correct_feed_price;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 6. ASSET DEACTIVATED: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_06_asset_deactivated_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    // 1. Failure: Asset is marked Suspended/Deactivated in registry
    ctx.is_asset_active = false;
    ctx.asset_status = "Suspended";
    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(result.is_err(), "Must fail closed when asset is deactivated");
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "ASSET_DEACTIVATED");

    // 2. Recovery: Asset is reactivated to Active
    ctx.is_asset_active = true;
    ctx.asset_status = "Active";
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 7. SHARIAH APPROVAL REVOKED: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_07_shariah_approval_revoked_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    // 1. Failure: Shariah approval status is Revoked
    ctx.shariah_status = ShariahStatus::Revoked;
    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_err(),
        "Must fail closed when Shariah status is revoked"
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "SHARIAH_APPROVAL_REVOKED");

    // 2. Recovery: Asset certified and Approved by Shariah supervisory board
    ctx.shariah_status = ShariahStatus::Approved;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 8. DEX QUOTE EXPIRED: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_08_dex_quote_expired_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    // 1. Failure: Quote expired 5 seconds ago
    ctx.quote_expires_at = now - 5;
    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(result.is_err(), "Must fail closed when DEX quote is expired");
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "DEX_QUOTE_EXPIRED");

    // 2. Recovery: Fresh quote obtained with valid TTL (30s remaining)
    ctx.quote_expires_at = now + 30;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 9. DEX UNAVAILABLE: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_09_dex_unavailable_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    // 1. Failure: Jupiter / Meteora quote service returns 504 Gateway Timeout
    let dex_down: Result<(), String> =
        Err("504 Gateway Timeout: Jupiter aggregator unreachable".to_string());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_down,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(result.is_err(), "Must fail closed when DEX is unavailable");
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "DEX_UNAVAILABLE");
    assert_eq!(failure.http_status_code(), 503);

    // 2. Recovery: DEX aggregator recovers and returns executable route
    let dex_up: Result<(), String> = Ok(());
    ctx.dex_result = &dex_up;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 10. SIGNER UNAVAILABLE: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_10_signer_unavailable_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    // 1. Failure: Remote HSM / KMS signer is offline or key is detached
    ctx.is_signer_available = false;
    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_err(),
        "Must fail closed when signer is unavailable"
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "SIGNER_UNAVAILABLE");
    assert_eq!(failure.http_status_code(), 503);

    // 2. Recovery: Signer reconnects and becomes available
    ctx.is_signer_available = true;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 11. TRANSACTION SUBMISSION FAILURE: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_11_transaction_submission_failure_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let db_ok: Result<(), String> = Ok(());

    // 1. Failure: Solana RPC node dropped connection on send_transaction
    let sub_err: Result<String, String> =
        Err("RPC TCP connection reset by peer during send_transaction".to_string());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_err,
        &db_ok,
        exec_id,
        now,
    );

    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_err(),
        "Must fail closed on transaction submission failure"
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "TRANSACTION_SUBMISSION_FAILURE");
    assert_eq!(failure.http_status_code(), 503);

    // 2. Recovery: RPC connection restored, transaction broadcast succeeds
    let sub_ok: Result<String, String> =
        Ok("5KzYmQhN7vL3...confirmed_tx_sig".to_string());
    ctx.submission_result = &sub_ok;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 12. TRANSACTION CONFIRMATION TIMEOUT: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_12_transaction_confirmation_timeout_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    // 1. Failure: Confirmation timed out (30s expired without cluster commitment)
    ctx.is_confirmed = false;
    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_err(),
        "Must fail closed on transaction confirmation timeout"
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "TRANSACTION_CONFIRMATION_TIMEOUT");
    assert_eq!(failure.http_status_code(), 504);

    // 2. Recovery: Confirmation confirmed by RPC cluster
    ctx.is_confirmed = true;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 13. DUPLICATE EXECUTION REQUEST: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_13_duplicate_execution_request_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    // 1. Failure: Duplicate execution request with previously confirmed execution_id
    ctx.is_duplicate = true;
    ctx.existing_status = Some("confirmed");
    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_err(),
        "Must fail closed on duplicate execution request"
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "DUPLICATE_EXECUTION_REQUEST");
    assert_eq!(failure.http_status_code(), 409);

    // 2. Recovery: Fresh unique execution request provided
    ctx.execution_id = Uuid::new_v4();
    ctx.is_duplicate = false;
    ctx.existing_status = None;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 14. DATABASE UNAVAILABLE: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_14_database_unavailable_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());

    // 1. Failure: PostgreSQL connection pool exhausted / socket closed
    let db_err: Result<(), String> =
        Err("FATAL: terminating connection due to administrator command".to_string());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_ok,
        &db_err,
        exec_id,
        now,
    );

    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_err(),
        "Must fail closed when database is unavailable"
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "DATABASE_UNAVAILABLE");
    assert_eq!(failure.http_status_code(), 503);

    // 2. Recovery: Database reconnects and writes succeed
    let db_ok: Result<(), String> = Ok(());
    ctx.db_result = &db_ok;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 15. WEBSOCKET CLIENTS DISCONNECTED: FAIL CLOSED AND RECOVER
// =============================================================================
#[test]
fn test_15_websocket_clients_disconnected_fails_closed_and_recovers() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();
    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> = Ok("5K...sig".to_string());
    let db_ok: Result<(), String> = Ok(());

    let mut ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    // 1. Failure: WebSocket client disconnected when streaming delivery is required
    ctx.is_ws_connected = false;
    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_err(),
        "Must fail closed when required WebSocket client stream is disconnected"
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.error_code(), "WEBSOCKET_CLIENT_DISCONNECTED");

    // 2. Recovery: WebSocket client reconnects and streams updates cleanly
    ctx.is_ws_connected = true;
    assert!(guard.evaluate_pipeline_safety(&ctx).is_ok());
}

// =============================================================================
// 16. END-TO-END CLEAN PIPELINE EXECUTION (ALL 15 INVARIANTS PASS)
// =============================================================================
#[test]
fn test_16_all_15_invariants_pass_clean_pipeline_executes_successfully() {
    let guard = TradingPipelineSafetyGuard::new();
    let now = 1_700_000_000;
    let exec_id = Uuid::new_v4();

    let price: Result<MarketPriceUpdate, String> =
        Ok(create_valid_price_update("NVDA", "0xfeed1234", now));
    let dex_ok: Result<(), String> = Ok(());
    let sub_ok: Result<String, String> =
        Ok("5KzYmQhN7vL3x5P1m9D8r4T7w6Q2a4S1c9V8b7N6m5".to_string());
    let db_ok: Result<(), String> = Ok(());

    let ctx = create_baseline_valid_context(
        "NVDA",
        "0xfeed1234",
        &price,
        &dex_ok,
        &sub_ok,
        &db_ok,
        exec_id,
        now,
    );

    let result = guard.evaluate_pipeline_safety(&ctx);
    assert!(
        result.is_ok(),
        "Clean pipeline must execute and complete when all 15 invariants hold"
    );
    assert_eq!(result.unwrap(), PipelineState::Completed);
}
