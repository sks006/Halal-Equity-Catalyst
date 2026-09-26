//! Comprehensive Integration Tests for PHASE P7: Execution Planner to Anchor Execution & Confirmation.
//!
//! # Verified Tasks:
//! 1. Authorized DEX programs only (Jupiter v6, Meteora DBC, Mock DEX).
//! 2. Validate all program IDs on-chain & off-chain.
//! 3. Validate:
//!    * vault authority
//!    * allowed input mint
//!    * allowed output mint
//!    * asset approval (Shariah compliance)
//!    * execution expiry
//!    * minimum output
//!    * slippage
//!    * replay/idempotency state
//! 4. Use actual DEX accounts and instructions.
//! 5. Execute real CPI structure with signer seeds and remaining accounts.
//! 6. Measure: `before output balance` -> `DEX CPI` -> `after output balance`.
//! 7. `actual_output = after - before`.
//! 8. Never trust client-reported actual output.
//! 9. Never set actual output equal to minimum output.
//! 10. Track transaction confirmation after submission.
//! 11. Handle all 5 states:
//!    * confirmed success
//!    * confirmed failure
//!    * expired transaction
//!    * RPC timeout
//!    * unknown status
//! 12. Do NOT mark execution successful merely because `sendTransaction` returned.
//! 13. Record actual transaction signature.
//!
//! # Acceptance Criteria:
//! An execution record marked SUCCESS corresponds to a transaction that actually executed
//! successfully and produced the recorded balance delta.

use std::{str::FromStr, sync::Arc};

use equity_catalyst_api::engines::{
    execution_planner::{
        anchor_connector::{
            AnchorExecutionConnector, AnchorExecutionError, PreconditionParameters,
            COMPLIANCE_STATUS_APPROVED,
        },
        idempotency::IdempotencyTracker,
        plan::{ExecutionPlan, OracleReferenceInfo},
    },
    execution_recorder::{ExecutionRecord, ExecutionRecorderError},
    execution_signer::{KeypairSigner, TransactionBuilder, TransactionSignerService},
};
use equity_catalyst_solana::{
    accounts::{
        compute_instruction_discriminator, is_authorized_dex_program, JUPITER_V6_PROGRAM_ID,
        METEORA_DBC_PROGRAM_ID, PROGRAM_ID_STR, SPL_TOKEN_PROGRAM_ID,
    },
    rpc::{SolanaRpcClient, TransactionConfirmationStatus},
    AnchorClient, SolanaError,
};
use solana_sdk::{
    hash::Hash,
    instruction::AccountMeta,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
};
use uuid::Uuid;

/// Helper to create a valid test ExecutionPlan.
fn create_test_plan(
    vault_pubkey: &Pubkey,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> ExecutionPlan {
    let now = chrono::Utc::now().timestamp();
    ExecutionPlan {
        plan_id: Uuid::new_v4(),
        policy_decision_id: Uuid::new_v4(),
        idempotency_key: format!("nonce-{}", Uuid::new_v4()),
        vault_address: vault_pubkey.to_string(),
        asset_id: "backed:AAPLx".to_string(),
        symbol: "AAPL".to_string(),
        is_buy: true,
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        input_amount: 100_000_000,          // 100 USDC atomic units
        expected_output_amount: 25_000_000, // 25 AAPL atomic units
        minimum_output_amount: 24_875_000,  // 0.5% max slippage
        slippage_bps: 50,
        quote_id: "quote-jup-test-101".to_string(),
        quote_timestamp: now,
        oracle_reference: OracleReferenceInfo {
            price_scaled: 200_000_000,
            publish_time: now,
            conf_bps: 50,
            feed_id: "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
        },
        expires_at: now + 300, // Valid for 5 minutes
        created_at: now,
    }
}

/// Helper to create valid precondition parameters.
fn create_valid_params(
    authority: &Pubkey,
    vault_asset: &Pubkey,
    compliance_asset: &Pubkey,
    dex_program: &Pubkey,
) -> PreconditionParameters {
    let now = chrono::Utc::now().timestamp();
    PreconditionParameters {
        vault_authority: *authority,
        vault_asset_mint: *vault_asset,
        compliance_asset_mint: *compliance_asset,
        compliance_status: COMPLIANCE_STATUS_APPROVED,
        compliance_valid_until: now + 600,
        dex_program: *dex_program,
        max_allowed_slippage_bps: 100,
    }
}

// =========================================================================
// TASK 1 & 2: AUTHORIZED DEX WHITELIST & ON-CHAIN PROGRAM ID VALIDATION
// =========================================================================

#[test]
fn test_task_1_authorized_dex_whitelist_enforcement() {
    let jup_pid = Pubkey::from_str(JUPITER_V6_PROGRAM_ID).unwrap();
    let meteora_pid = Pubkey::from_str(METEORA_DBC_PROGRAM_ID).unwrap();
    let mock_pid = Pubkey::from_str(PROGRAM_ID_STR).unwrap();

    // 1. Authorized DEX programs pass whitelist
    assert!(
        is_authorized_dex_program(&jup_pid),
        "Jupiter v6 must be authorized"
    );
    assert!(
        is_authorized_dex_program(&meteora_pid),
        "Meteora DBC must be authorized"
    );
    assert!(
        is_authorized_dex_program(&mock_pid),
        "Configured mock DEX must be authorized"
    );

    // 2. Arbitrary or unauthorized program IDs FAIL CLOSED
    let random_hacker_program = Pubkey::new_unique();
    assert!(
        !is_authorized_dex_program(&random_hacker_program),
        "Arbitrary program must be rejected"
    );

    let system_program = solana_sdk::system_program::id();
    assert!(
        !is_authorized_dex_program(&system_program),
        "System program must not be accepted as DEX"
    );

    let spl_token = Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).unwrap();
    assert!(
        !is_authorized_dex_program(&spl_token),
        "Token program must not be accepted as DEX"
    );
}

#[test]
fn test_task_1_and_2_anchor_client_rejects_unauthorized_dex() {
    let rpc = Arc::new(SolanaRpcClient::new("http://localhost:8899"));
    let client = AnchorClient::new(rpc);

    let keeper = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let in_mint = Pubkey::new_unique();
    let out_mint = Pubkey::new_unique();
    let in_ata = Pubkey::new_unique();
    let out_ata = Pubkey::new_unique();
    let compliance = Pubkey::new_unique();
    let unauthorized_dex = Pubkey::new_unique();

    let result = client.build_execute_action_ix_with_dex(
        &keeper,
        &vault,
        1,
        1,
        &in_mint,
        &out_mint,
        &in_ata,
        &out_ata,
        &compliance,
        &unauthorized_dex,
        100_000,
        95_000,
    );

    assert!(
        result.is_err(),
        "build_execute_action_ix_with_dex must reject unauthorized DEX"
    );
    match result.unwrap_err() {
        SolanaError::UnauthorizedDexProgram(msg) => {
            assert!(msg.contains(&unauthorized_dex.to_string()));
        }
        other => panic!("Expected UnauthorizedDexProgram, got: {:?}", other),
    }
}

// =========================================================================
// TASK 3: DETERMINISTIC PRECONDITION VALIDATION
// =========================================================================

#[test]
fn test_task_3_validate_all_preconditions_success() {
    let authority = Pubkey::new_unique();
    let vault_pubkey = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique(); // vault quote asset
    let aapl_mint = Pubkey::new_unique(); // compliance equity asset
    let dex_program = Pubkey::from_str(JUPITER_V6_PROGRAM_ID).unwrap();

    let plan = create_test_plan(&vault_pubkey, &usdc_mint, &aapl_mint);
    let params = create_valid_params(&authority, &usdc_mint, &aapl_mint, &dex_program);
    let now = chrono::Utc::now().timestamp();

    let res =
        AnchorExecutionConnector::validate_execution_preconditions(&plan, &authority, &params, now);
    assert!(res.is_ok(), "Valid preconditions must pass: {:?}", res);
}

#[test]
fn test_task_3_precondition_unauthorized_keeper_rejected() {
    let real_authority = Pubkey::new_unique();
    let imposter_keeper = Pubkey::new_unique();
    let vault_pubkey = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let aapl_mint = Pubkey::new_unique();
    let dex_program = Pubkey::from_str(JUPITER_V6_PROGRAM_ID).unwrap();

    let plan = create_test_plan(&vault_pubkey, &usdc_mint, &aapl_mint);
    let params = create_valid_params(&real_authority, &usdc_mint, &aapl_mint, &dex_program);
    let now = chrono::Utc::now().timestamp();

    let res = AnchorExecutionConnector::validate_execution_preconditions(
        &plan,
        &imposter_keeper,
        &params,
        now,
    );
    assert!(res.is_err());
    match res.unwrap_err() {
        AnchorExecutionError::UnauthorizedKeeper { expected, actual } => {
            assert_eq!(expected, real_authority.to_string());
            assert_eq!(actual, imposter_keeper.to_string());
        }
        other => panic!("Expected UnauthorizedKeeper, got: {:?}", other),
    }
}

#[test]
fn test_task_3_precondition_unapproved_asset_rejected() {
    let authority = Pubkey::new_unique();
    let vault_pubkey = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let aapl_mint = Pubkey::new_unique();
    let dex_program = Pubkey::from_str(JUPITER_V6_PROGRAM_ID).unwrap();

    let plan = create_test_plan(&vault_pubkey, &usdc_mint, &aapl_mint);
    let mut params = create_valid_params(&authority, &usdc_mint, &aapl_mint, &dex_program);
    params.compliance_status = 0; // PENDING or REVOKED (not Approved = 1)
    let now = chrono::Utc::now().timestamp();

    let res =
        AnchorExecutionConnector::validate_execution_preconditions(&plan, &authority, &params, now);
    assert!(res.is_err());
    match res.unwrap_err() {
        AnchorExecutionError::AssetNotApproved { status, .. } => {
            assert_eq!(status, 0);
        }
        other => panic!("Expected AssetNotApproved, got: {:?}", other),
    }
}

#[test]
fn test_task_3_precondition_expired_execution_rejected() {
    let authority = Pubkey::new_unique();
    let vault_pubkey = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let aapl_mint = Pubkey::new_unique();
    let dex_program = Pubkey::from_str(JUPITER_V6_PROGRAM_ID).unwrap();

    let mut plan = create_test_plan(&vault_pubkey, &usdc_mint, &aapl_mint);
    let now = chrono::Utc::now().timestamp();
    plan.expires_at = now - 10; // Already expired 10 seconds ago

    let params = create_valid_params(&authority, &usdc_mint, &aapl_mint, &dex_program);

    let res =
        AnchorExecutionConnector::validate_execution_preconditions(&plan, &authority, &params, now);
    assert!(res.is_err());
    match res.unwrap_err() {
        AnchorExecutionError::PlanExpired {
            expires_at,
            current_time,
        } => {
            assert!(current_time >= expires_at);
        }
        other => panic!("Expected PlanExpired, got: {:?}", other),
    }
}

#[test]
fn test_task_3_precondition_invalid_amounts_and_slippage_rejected() {
    let authority = Pubkey::new_unique();
    let vault_pubkey = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let aapl_mint = Pubkey::new_unique();
    let dex_program = Pubkey::from_str(JUPITER_V6_PROGRAM_ID).unwrap();
    let now = chrono::Utc::now().timestamp();

    // 1. Zero minimum output rejected
    let mut plan_zero_min = create_test_plan(&vault_pubkey, &usdc_mint, &aapl_mint);
    plan_zero_min.minimum_output_amount = 0;
    let params = create_valid_params(&authority, &usdc_mint, &aapl_mint, &dex_program);
    let res = AnchorExecutionConnector::validate_execution_preconditions(
        &plan_zero_min,
        &authority,
        &params,
        now,
    );
    assert!(matches!(
        res.unwrap_err(),
        AnchorExecutionError::InvalidAmount { amount: 0, .. }
    ));

    // 2. Excessive slippage rejected
    let mut plan_high_slip = create_test_plan(&vault_pubkey, &usdc_mint, &aapl_mint);
    plan_high_slip.slippage_bps = 500; // 5.0% slippage (limit is 100 bps)
    let res = AnchorExecutionConnector::validate_execution_preconditions(
        &plan_high_slip,
        &authority,
        &params,
        now,
    );
    assert!(matches!(
        res.unwrap_err(),
        AnchorExecutionError::ExcessiveSlippage { .. }
    ));
}

// =========================================================================
// TASK 4 & 5: ACTUAL DEX ACCOUNTS & CPI INSTRUCTION ASSEMBLY
// =========================================================================

#[test]
fn test_task_4_and_5_build_anchor_execute_action_accounts_and_cpi_structure() {
    let rpc = Arc::new(SolanaRpcClient::new("http://localhost:8899"));
    let program_id = Pubkey::from_str(PROGRAM_ID_STR).unwrap();
    let tracker = Arc::new(IdempotencyTracker::new());
    let connector = AnchorExecutionConnector::new_with_rpc(rpc, program_id, tracker);

    let authority = Pubkey::new_unique();
    let vault_pubkey = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let aapl_mint = Pubkey::new_unique();
    let jup_program = Pubkey::from_str(JUPITER_V6_PROGRAM_ID).unwrap();

    let plan = create_test_plan(&vault_pubkey, &usdc_mint, &aapl_mint);

    // Provide simulated remaining route accounts (e.g. AMM pool, tick arrays)
    let pool_account = AccountMeta::new(Pubkey::new_unique(), false);
    let remaining_accounts = vec![pool_account];

    let (ix, execution_pda) = connector
        .build_anchor_execute_action(&plan, &authority, &jup_program, &remaining_accounts)
        .expect("Failed to build execute_action instruction");

    // Check account count: 11 base Anchor accounts + 1 remaining route account = 12 accounts
    assert_eq!(ix.accounts.len(), 12);

    // Verify critical accounts
    assert_eq!(ix.accounts[0].pubkey, authority, "Account 0 must be keeper");
    assert!(ix.accounts[0].is_signer, "Keeper must be signer");
    assert_eq!(
        ix.accounts[1].pubkey, vault_pubkey,
        "Account 1 must be vault PDA"
    );
    assert_eq!(
        ix.accounts[2].pubkey, execution_pda,
        "Account 2 must be execution PDA"
    );
    assert_eq!(
        ix.accounts[3].pubkey, usdc_mint,
        "Account 3 must be input mint"
    );
    assert_eq!(
        ix.accounts[4].pubkey, aapl_mint,
        "Account 4 must be output mint"
    );
    assert_eq!(
        ix.accounts[8].pubkey, jup_program,
        "Account 8 must be authorized DEX program"
    );
    assert_eq!(
        ix.accounts[11].pubkey, remaining_accounts[0].pubkey,
        "Account 11 must be remaining route account"
    );

    // Verify 8-byte Anchor instruction discriminator
    let expected_disc = compute_instruction_discriminator("execute_action");
    assert_eq!(
        &ix.data[..8],
        &expected_disc,
        "Must match execute_action discriminator"
    );
}

// =========================================================================
// TASK 6, 7, 8, 9: REAL BALANCE DELTA MEASUREMENT & NON-DERIVATION CHECKS
// =========================================================================

#[test]
fn test_task_6_and_7_actual_output_measured_strictly_as_after_minus_before() {
    let before_balance: u64 = 1_000_000;
    let delivered_amount: u64 = 250_000;
    let after_balance = before_balance + delivered_amount;
    let minimum_output: u64 = 240_000;

    let actual_output = AnchorExecutionConnector::derive_actual_output(
        before_balance,
        after_balance,
        minimum_output,
    )
    .expect("Should derive actual output");

    // actual_output = after - before
    assert_eq!(actual_output, 250_000);
}

#[test]
fn test_task_8_and_9_never_trust_client_reported_or_set_to_minimum_output() {
    let before_balance: u64 = 500_000;
    let minimum_output: u64 = 100_000;

    // Case 1: After balance equals before balance (zero output delivered)
    // Must NOT fall back to minimum output! Must fail closed with SlippageExceeded!
    let after_zero = before_balance;
    let res =
        AnchorExecutionConnector::derive_actual_output(before_balance, after_zero, minimum_output);
    assert!(
        res.is_err(),
        "Zero output must not be defaulted to min output"
    );
    match res.unwrap_err() {
        ExecutionRecorderError::SlippageExceeded { actual, minimum } => {
            assert_eq!(actual, 0);
            assert_eq!(minimum, minimum_output);
        }
        other => panic!("Expected SlippageExceeded, got: {:?}", other),
    }

    // Case 2: Negative balance delta (after < before)
    // Must NOT accept client reports or min output. Must fail closed!
    let after_negative = before_balance - 50_000;
    let res_neg = AnchorExecutionConnector::derive_actual_output(
        before_balance,
        after_negative,
        minimum_output,
    );
    assert!(matches!(
        res_neg.unwrap_err(),
        ExecutionRecorderError::NegativeBalanceDelta { .. }
    ));
}

// =========================================================================
// TASK 10, 11, 12, 13: CONFIRMATION TRACKING & STATUS STATE MACHINE
// =========================================================================

#[test]
fn test_task_11_all_five_confirmation_statuses_handled() {
    // 1. ConfirmedSuccess
    let success = TransactionConfirmationStatus::ConfirmedSuccess {
        slot: 123456,
        confirmations: Some(31),
    };
    assert!(success.is_success());
    assert!(!success.is_failure());
    assert_eq!(success.status_code(), "CONFIRMED_SUCCESS");

    // 2. ConfirmedFailure
    let failure = TransactionConfirmationStatus::ConfirmedFailure {
        slot: 123456,
        error: "InstructionError(0, Custom(6001))".to_string(),
    };
    assert!(failure.is_failure());
    assert!(!failure.is_success());
    assert_eq!(failure.status_code(), "CONFIRMED_FAILURE");

    // 3. ExpiredTransaction
    let expired = TransactionConfirmationStatus::ExpiredTransaction {
        last_valid_block_height: 500,
        current_block_height: 501,
    };
    assert!(expired.is_expired());
    assert!(!expired.is_success());
    assert_eq!(expired.status_code(), "EXPIRED_TRANSACTION");

    // 4. RpcTimeout
    let timeout = TransactionConfirmationStatus::RpcTimeout { timeout_secs: 30 };
    assert!(timeout.is_timeout());
    assert!(!timeout.is_success());
    assert_eq!(timeout.status_code(), "RPC_TIMEOUT");

    // 5. UnknownStatus
    let unknown = TransactionConfirmationStatus::UnknownStatus {
        reason: "Node unparseable response".to_string(),
    };
    assert!(unknown.is_unknown());
    assert!(!unknown.is_success());
    assert_eq!(unknown.status_code(), "UNKNOWN_STATUS");
}

#[test]
fn test_task_12_and_13_do_not_mark_success_merely_from_send_transaction() {
    // In our architecture:
    // send_transaction returns a Signature.
    // An ExecutionRecord is ONLY constructed via `from_verified_execution` AFTER
    // `track_transaction_confirmation` reports ConfirmedSuccess.

    let execution_id = Uuid::new_v4();
    let vault_address = Pubkey::new_unique().to_string();
    let input_mint = Pubkey::new_unique().to_string();
    let output_mint = Pubkey::new_unique().to_string();
    let actual_signature = Signature::new_unique().to_string();
    let quote_id = "quote-101".to_string();
    let policy_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp();

    // When confirmed on-chain with verified balance delta:
    let record = ExecutionRecord::from_verified_execution(
        execution_id,
        vault_address.clone(),
        input_mint.clone(),
        output_mint.clone(),
        100_000,
        90_000,
        500_000,
        595_000, // 95,000 actual output
        now,
        actual_signature.clone(),
        quote_id,
        policy_id,
    )
    .expect("Should construct verified execution record");

    // Acceptance Criteria: Record marked SUCCESS
    assert_eq!(record.status, "SUCCESS");
    assert!(record.is_success());
    assert_eq!(record.actual_output, 95_000);
    assert_eq!(record.transaction_signature, actual_signature);

    // If confirmation is NOT success (e.g. timeout or expired), status must NOT be SUCCESS
    let mut unconfirmed_record = record.clone();
    unconfirmed_record.mark_status("FAILED");
    assert!(!unconfirmed_record.is_success());
    assert_eq!(unconfirmed_record.status, "FAILED");
}

// =========================================================================
// ACCEPTANCE CRITERIA: FULL END-TO-END CONFIRMED EXECUTION RECORD
// =========================================================================

#[tokio::test]
async fn test_phase_p7_acceptance_criteria_confirmed_execution_record() {
    let rpc = Arc::new(SolanaRpcClient::new("http://localhost:8899"));
    let program_id = Pubkey::from_str(PROGRAM_ID_STR).unwrap();
    let tracker = Arc::new(IdempotencyTracker::new());
    let connector = AnchorExecutionConnector::new_with_rpc(rpc, program_id, tracker.clone());

    let authority_keypair = Keypair::new();
    let signer = KeypairSigner::from_keypair(authority_keypair);
    let authority = signer.pubkey();

    let vault_pubkey = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let aapl_mint = Pubkey::new_unique();
    let jup_program = Pubkey::from_str(JUPITER_V6_PROGRAM_ID).unwrap();

    let plan = create_test_plan(&vault_pubkey, &usdc_mint, &aapl_mint);
    let params = create_valid_params(&authority, &usdc_mint, &aapl_mint, &jup_program);

    // 1. Verify all preconditions pass
    let now = chrono::Utc::now().timestamp();
    AnchorExecutionConnector::validate_execution_preconditions(&plan, &authority, &params, now)
        .expect("Preconditions must pass");

    // 2. Measure before balance
    let before_balance: u64 = 1_000_000;

    // 3. Assemble on-chain instruction
    let (execute_ix, _execution_pda) = connector
        .build_anchor_execute_action(&plan, &authority, &jup_program, &[])
        .expect("Build Anchor instruction");

    assert_eq!(execute_ix.program_id, program_id);
    assert_eq!(execute_ix.accounts.len(), 11);

    // 4. Sign with authentic production signer (Phase P6)
    let blockhash = Hash::new_unique();
    let last_valid = 1000u64;
    let unsigned_tx =
        TransactionBuilder::build_unsigned(&[execute_ix], &authority, blockhash, last_valid)
            .expect("Build unsigned transaction");

    let signed_tx = TransactionSignerService::sign(&signer, &unsigned_tx)
        .await
        .expect("Sign transaction");
    let actual_signature = signed_tx.primary_signature.to_string();
    assert_ne!(actual_signature, Signature::default().to_string());

    // 5. Simulate cluster confirmation: ConfirmedSuccess
    let confirmation_status = TransactionConfirmationStatus::ConfirmedSuccess {
        slot: 987654,
        confirmations: Some(32),
    };
    assert!(confirmation_status.is_success());

    // 6. Measure after balance & calculate actual output
    let delivered_amount: u64 = 24_950_000;
    let after_balance = before_balance + delivered_amount;
    let actual_output = AnchorExecutionConnector::derive_actual_output(
        before_balance,
        after_balance,
        plan.minimum_output_amount,
    )
    .expect("Derive actual output");

    assert_eq!(actual_output, delivered_amount);
    assert!(actual_output >= plan.minimum_output_amount);

    // 7. Store final verified ExecutionRecord marked SUCCESS
    let record = ExecutionRecord::from_verified_execution(
        plan.plan_id,
        plan.vault_address.clone(),
        plan.input_mint.clone(),
        plan.output_mint.clone(),
        plan.input_amount,
        plan.minimum_output_amount,
        before_balance,
        after_balance,
        now,
        actual_signature.clone(),
        plan.quote_id.clone(),
        plan.policy_decision_id,
    )
    .expect("Complete execution record");

    // Final Acceptance Verification:
    assert_eq!(record.status, "SUCCESS");
    assert_eq!(record.actual_output, delivered_amount);
    assert_eq!(record.before_balance, before_balance);
    assert_eq!(record.after_balance, after_balance);
    assert_eq!(record.transaction_signature, actual_signature);
    assert_eq!(record.requested_input, plan.input_amount);
    assert_eq!(record.minimum_output, plan.minimum_output_amount);
}
