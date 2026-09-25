//! Comprehensive Integration & Unit Test Suite for PHASE 14:
//! DEX Execution Recording Using Token-Account Balance Deltas.
//!
//! # Objective
//! Record the actual result of a DEX execution strictly using token-account balance deltas.
//!
//! # Tasks & Invariants Verified:
//! 1. Before CPI: record relevant output token balance (`before_balance`).
//! 2. Execute DEX CPI.
//! 3. After CPI: read output token balance again (`after_balance`).
//! 4. Calculate: `actual_output_amount = after_balance - before_balance`.
//! 5. Do NOT derive actual output from:
//!    - minimum output
//!    - expected output
//!    - oracle price
//!    - quote estimate
//!    - client input
//! 6. Validate that: `after_balance >= before_balance` (unless explicitly allowing negative balance delta semantics).
//! 7. Record:
//!    - requested input
//!    - minimum output
//!    - actual output
//!    - execution timestamp
//!    - transaction signature
//!    - quote ID
//!    - policy decision ID
//! 8. Rejection of slippage violation (`actual_output < minimum_output`).
//! 9. End-to-end integration with ExecutionPlan and ExecutionOutcome.

use chrono::Utc;
use equity_catalyst_api::engines::execution_planner::{ExecutionPlan, OracleReferenceInfo};
use equity_catalyst_api::engines::execution_recorder::{
    ExecutionBalanceTracker, ExecutionRecord, ExecutionRecorderError,
};
use equity_catalyst_api::models::ExecutionModel;
use equity_catalyst_api::services::execution_engine_service::{
    calculate_slippage_drift_bps, ExecutionOutcome, ExecutionRequest,
};
use solana_sdk::pubkey::Pubkey;
use uuid::Uuid;

#[test]
fn test_balance_delta_actual_output_derivation_success() {
    let execution_id = Uuid::new_v4();
    let policy_decision_id = Uuid::new_v4();
    let requested_input: u64 = 1_000_000; // 1.0 USDC
    let minimum_output: u64 = 980_000; // 0.98 AAPL min
    let before_balance: u64 = 10_000_000; // Pre-CPI balance
    let after_balance: u64 = 10_987_654; // Post-CPI balance
    let execution_timestamp: i64 = 1720000000;
    let tx_sig = "5TxSigActualBalanceDelta111111111111111111111111111111111111111111111111111111111111111111".to_string();
    let quote_id = "quote-jupiter-dbc-001".to_string();

    let record = ExecutionRecord::from_balance_delta(
        execution_id,
        "Vault111111111111111111111111111111111111111".to_string(),
        "USDC111111111111111111111111111111111111111".to_string(),
        "AAPL111111111111111111111111111111111111111".to_string(),
        requested_input,
        minimum_output,
        before_balance,
        after_balance,
        execution_timestamp,
        tx_sig.clone(),
        quote_id.clone(),
        policy_decision_id,
        false,
    )
    .expect("Valid balance delta must create ExecutionRecord");

    // REQUIREMENT: actual_output_amount = after_balance - before_balance
    let expected_delta = after_balance - before_balance;
    assert_eq!(expected_delta, 987_654);
    assert_eq!(record.actual_output, 987_654);

    // REQUIREMENT: Do NOT derive actual output from minimum output
    assert_ne!(record.actual_output, minimum_output);
    assert_ne!(record.actual_output, 990_000); // Expected output
    assert_ne!(record.actual_output, requested_input); // Input amount

    // REQUIREMENT: Verify all 7 required recorded fields
    assert_eq!(record.requested_input, requested_input);
    assert_eq!(record.minimum_output, minimum_output);
    assert_eq!(record.actual_output, 987_654);
    assert_eq!(record.execution_timestamp, execution_timestamp);
    assert_eq!(record.transaction_signature, tx_sig);
    assert_eq!(record.quote_id, quote_id);
    assert_eq!(record.policy_decision_id, policy_decision_id);

    // Contextual sanity checks
    assert_eq!(record.before_balance, before_balance);
    assert_eq!(record.after_balance, after_balance);
    assert_eq!(record.status, "confirmed");
}

#[test]
fn test_negative_balance_delta_rejected_when_not_explicitly_supported() {
    let execution_id = Uuid::new_v4();
    let policy_decision_id = Uuid::new_v4();
    let before_balance: u64 = 5_000_000;
    let after_balance: u64 = 4_990_000; // Output balance dropped by 10,000

    let err = ExecutionRecord::from_balance_delta(
        execution_id,
        "Vault111111111111111111111111111111111111111".to_string(),
        "InMint".to_string(),
        "OutMint".to_string(),
        1_000_000,
        900_000,
        before_balance,
        after_balance,
        1720000000,
        "sig123".to_string(),
        "q-01".to_string(),
        policy_decision_id,
        false, // Disallow negative delta
    )
    .unwrap_err();

    assert_eq!(
        err,
        ExecutionRecorderError::NegativeBalanceDelta {
            before: 5_000_000,
            after: 4_990_000,
        }
    );
}

#[test]
fn test_slippage_violation_rejected() {
    let execution_id = Uuid::new_v4();
    let policy_decision_id = Uuid::new_v4();
    let before_balance: u64 = 2_000_000;
    let after_balance: u64 = 2_940_000; // Delta: 940_000
    let minimum_output: u64 = 950_000; // Min output is 950_000 > 940_000

    let err = ExecutionRecord::from_balance_delta(
        execution_id,
        "Vault111111111111111111111111111111111111111".to_string(),
        "InMint".to_string(),
        "OutMint".to_string(),
        1_000_000,
        minimum_output,
        before_balance,
        after_balance,
        1720000000,
        "sig123".to_string(),
        "q-01".to_string(),
        policy_decision_id,
        false,
    )
    .unwrap_err();

    assert_eq!(
        err,
        ExecutionRecorderError::SlippageExceeded {
            actual: 940_000,
            minimum: 950_000,
        }
    );
}

#[test]
fn test_do_not_derive_from_estimates_oracle_or_client_input() {
    let execution_id = Uuid::new_v4();
    let policy_decision_id = Uuid::new_v4();

    let client_claimed_amount: u64 = 1_000_000;
    let quote_estimate: u64 = 992_000;
    let min_output: u64 = 980_000;
    let oracle_derived_output: u64 = 995_000;

    // Real on-chain vault token accounts:
    let before_balance: u64 = 50_000_000;
    let after_balance: u64 = 50_983_210; // Real on-chain balance delta is 983,210

    let record = ExecutionRecord::from_balance_delta(
        execution_id,
        "Vault111111111111111111111111111111111111111".to_string(),
        "InMint".to_string(),
        "OutMint".to_string(),
        1_000_000,
        min_output,
        before_balance,
        after_balance,
        1720000000,
        "sig".to_string(),
        "q-test".to_string(),
        policy_decision_id,
        false,
    )
    .expect("Should derive actual output from balance delta");

    // Strictly matches balance delta
    assert_eq!(record.actual_output, 983_210);

    // Must NOT match any client claim or quote estimate
    assert_ne!(record.actual_output, client_claimed_amount);
    assert_ne!(record.actual_output, quote_estimate);
    assert_ne!(record.actual_output, min_output);
    assert_ne!(record.actual_output, oracle_derived_output);
}

#[test]
fn test_execution_balance_tracker_lifecycle_and_checks() {
    let output_ata = Pubkey::new_unique();
    let policy_decision_id = Uuid::new_v4();
    let mut tracker = ExecutionBalanceTracker::new(
        output_ata,
        2_000_000,
        1_950_000,
        "quote-dbc-lifecycle".to_string(),
        policy_decision_id,
    );

    // 1. Missing pre-balance fails
    assert_eq!(
        tracker.calculate_actual_output(false).unwrap_err(),
        ExecutionRecorderError::MissingBeforeBalance
    );

    // 2. Pre-CPI balance recorded
    tracker.record_before_balance(100_000_000);

    // Missing post-balance fails
    assert_eq!(
        tracker.calculate_actual_output(false).unwrap_err(),
        ExecutionRecorderError::MissingAfterBalance
    );

    // 3. Post-CPI balance recorded
    tracker.record_after_balance(101_975_000);

    // 4. Calculate actual output
    let actual_out = tracker.calculate_actual_output(false).unwrap();
    assert_eq!(actual_out, 1_975_000);

    // 5. Complete record
    let record = tracker
        .complete(
            Uuid::new_v4(),
            "VaultAddress".to_string(),
            "InputMint".to_string(),
            "OutputMint".to_string(),
            "SolanaTxSig".to_string(),
            1725000000,
            false,
        )
        .expect("Tracker must complete successfully");

    assert_eq!(record.actual_output, 1_975_000);
    assert_eq!(record.requested_input, 2_000_000);
    assert_eq!(record.minimum_output, 1_950_000);
    assert_eq!(record.quote_id, "quote-dbc-lifecycle");
    assert_eq!(record.policy_decision_id, policy_decision_id);
    assert_eq!(record.transaction_signature, "SolanaTxSig");
    assert_eq!(record.execution_timestamp, 1725000000);
}

#[test]
fn test_execution_request_from_plan_preserves_identifiers() {
    let plan = ExecutionPlan {
        plan_id: Uuid::new_v4(),
        policy_decision_id: Uuid::new_v4(),
        idempotency_key: "idemp:test:001".to_string(),
        vault_address: "Vault111111111111111111111111111111111111111".to_string(),
        asset_id: "backed:AAPLx".to_string(),
        symbol: "AAPL".to_string(),
        is_buy: true,
        input_mint: "USDC111111111111111111111111111111111111111".to_string(),
        output_mint: "AAPL111111111111111111111111111111111111111".to_string(),
        input_amount: 5_000_000,
        expected_output_amount: 4_950_000,
        minimum_output_amount: 4_900_000,
        slippage_bps: 100,
        quote_id: "quote-jup-999".to_string(),
        quote_timestamp: 1720000000,
        oracle_reference: OracleReferenceInfo {
            price_scaled: 198_000_000,
            publish_time: 1720000000,
            conf_bps: 10,
            feed_id: "feed-aapl".to_string(),
        },
        expires_at: 1720000060,
        created_at: 1720000000,
    };

    let req = ExecutionRequest::from_plan(&plan);

    assert_eq!(req.execution_id, plan.plan_id);
    assert_eq!(req.vault_address, plan.vault_address);
    assert_eq!(req.amount_in, 5_000_000);
    assert_eq!(req.min_amount_out, 4_900_000);
    assert_eq!(req.amount_out_expected, 4_950_000);
    assert_eq!(req.quote_id, Some("quote-jup-999".to_string()));
    assert_eq!(req.policy_decision_id, Some(plan.policy_decision_id));
}

#[test]
fn test_execution_outcome_with_record_and_drift_calculation() {
    let execution_id = Uuid::new_v4();
    let policy_decision_id = Uuid::new_v4();
    let expected_output: u64 = 1_000_000;
    let actual_output: u64 = 990_000; // -1.0% drift = -100 bps

    let drift = calculate_slippage_drift_bps(expected_output, actual_output);
    assert_eq!(drift, -100);

    let record = ExecutionRecord::from_balance_delta(
        execution_id,
        "VaultAddress".to_string(),
        "InMint".to_string(),
        "OutMint".to_string(),
        1_000_000,
        980_000,
        10_000_000,
        10_990_000, // Delta 990_000
        1720000000,
        "tx_sig_test".to_string(),
        "quote_123".to_string(),
        policy_decision_id,
        false,
    )
    .unwrap();

    let outcome = ExecutionOutcome {
        execution_id,
        vault_address: "VaultAddress".to_string(),
        tx_signature: Some("tx_sig_test".to_string()),
        status: "confirmed".to_string(),
        amount_out_actual: Some(record.actual_output),
        slippage_drift_bps: Some(drift),
        simulation_success: true,
        error_message: None,
        confirmed: true,
        execution_record: Some(record),
        quote_id: Some("quote_123".to_string()),
        policy_decision_id: Some(policy_decision_id),
    };

    assert_eq!(outcome.amount_out_actual, Some(990_000));
    assert_eq!(outcome.slippage_drift_bps, Some(-100));
    assert_eq!(outcome.confirmed, true);
    let rec = outcome.execution_record.unwrap();
    assert_eq!(rec.actual_output, 990_000);
    assert_eq!(rec.quote_id, "quote_123");
    assert_eq!(rec.policy_decision_id, policy_decision_id);
}

#[test]
fn test_execution_model_optional_fields_serialization() {
    let exec_id = Uuid::new_v4();
    let dec_id = Uuid::new_v4();
    let model = ExecutionModel {
        execution_id: exec_id,
        vault_address: "VaultAddress".to_string(),
        event_id: None,
        action: "BUY_AAPL".to_string(),
        input_mint: "USDC".to_string(),
        output_mint: "AAPL".to_string(),
        amount_in: 1_000_000,
        amount_out_expected: 990_000,
        amount_out_actual: Some(992_000),
        slippage_bps: 100,
        tx_signature: Some("sig58".to_string()),
        status: "confirmed".to_string(),
        error_message: None,
        executed_at: Utc::now(),
        confirmed_at: Some(Utc::now()),
        quote_id: Some("quote-abc".to_string()),
        policy_decision_id: Some(dec_id),
        amount_out_min: Some(980_000),
    };

    let json = serde_json::to_string(&model).expect("Serialization must succeed");
    assert!(json.contains("quote-abc"));
    assert!(json.contains(&dec_id.to_string()));
    assert!(json.contains("992000"));

    let deserialized: ExecutionModel =
        serde_json::from_str(&json).expect("Deserialization must succeed");
    assert_eq!(deserialized.quote_id, Some("quote-abc".to_string()));
    assert_eq!(deserialized.policy_decision_id, Some(dec_id));
    assert_eq!(deserialized.amount_out_actual, Some(992_000));
}
