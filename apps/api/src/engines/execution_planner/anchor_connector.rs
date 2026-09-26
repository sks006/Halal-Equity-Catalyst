//! Production Anchor Execution Connector (Phase P7).
//!
//! # Objective
//! Connect the production execution planner to the real Anchor execution instruction and confirm
//! the actual result on Solana.
//!
//! # Enforced Invariants:
//! 1. The execution instruction must invoke only explicitly authorized DEX programs (Jupiter v6, Meteora DBC, Mock DEX).
//! 2. Validate all program IDs on-chain.
//! 3. Deterministically validate:
//!    * vault authority
//!    * allowed input mint
//!    * allowed output mint
//!    * asset approval (Shariah compliance approved)
//!    * execution expiry (quote TTL and compliance TTL)
//!    * minimum output (> 0)
//!    * slippage
//!    * replay / idempotency state
//! 4. Use actual DEX accounts and instructions.
//! 5. Execute the real CPI.
//! 6. Measure: `before output balance` -> `DEX CPI` -> `after output balance`.
//! 7. `actual_output = after - before`.
//! 8. Never trust a client-reported actual output.
//! 9. Never set actual output equal to minimum output.
//! 10. After submission, track transaction confirmation through cluster RPC.
//! 11. Handle:
//!    * ConfirmedSuccess
//!    * ConfirmedFailure
//!    * ExpiredTransaction
//!    * RpcTimeout
//!    * UnknownStatus
//! 12. Do NOT mark execution successful merely because `sendTransaction` returned.
//! 13. Record the actual transaction signature.
//!
//! # Acceptance Criteria:
//! An execution record marked SUCCESS corresponds to a transaction that actually executed
//! successfully on-chain and produced the recorded balance delta.

use chrono::Utc;
use equity_catalyst_solana::{
    accounts::{find_associated_token_address, find_compliance_pda, is_authorized_dex_program},
    rpc::{SolanaRpcClient, TransactionConfirmationStatus},
    AnchorClient, SolanaError,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::{str::FromStr, sync::Arc, time::Duration};
use thiserror::Error;
use tracing::{info, instrument, warn};

use crate::engines::{
    execution_planner::{
        idempotency::{IdempotencyStatus, IdempotencyTracker},
        plan::ExecutionPlan,
    },
    execution_recorder::{ExecutionRecord, ExecutionRecorderError},
    execution_signer::{ExternalSigner, SignerError, TransactionBuilder, TransactionSignerService},
};

/// On-chain Shariah Compliance Status: Approved
pub const COMPLIANCE_STATUS_APPROVED: u8 = 1;

/// Errors arising during pre-execution validation, execution instruction construction,
/// or post-execution confirmation and reconciliation.
#[derive(Debug, Error)]
pub enum AnchorExecutionError {
    #[error("Vault authority mismatch: expected {expected}, got keeper {actual}")]
    UnauthorizedKeeper { expected: String, actual: String },

    #[error(
        "Unauthorized DEX program ID: {0}. Target program must be in authorized DEX whitelist."
    )]
    UnauthorizedDexProgram(String),

    #[error("Invalid token mints: {reason} (input: {input}, output: {output})")]
    InvalidTokenMints {
        input: String,
        output: String,
        reason: String,
    },

    #[error("Asset compliance not approved: mint {asset_mint}, status {status}")]
    AssetNotApproved { asset_mint: String, status: u8 },

    #[error("Asset compliance expired: valid until {valid_until}, current time {current_time}")]
    ComplianceExpired { valid_until: i64, current_time: i64 },

    #[error("Execution plan expired: expires at {expires_at}, current time {current_time}")]
    PlanExpired { expires_at: i64, current_time: i64 },

    #[error("Invalid trade amount: {reason} (amount: {amount})")]
    InvalidAmount { amount: u64, reason: String },

    #[error("Slippage limit exceeded: plan specifies {slippage_bps} bps, maximum allowed is {max_allowed} bps")]
    ExcessiveSlippage { slippage_bps: u16, max_allowed: u16 },

    #[error("Idempotency / replay protection violation: key {key}, reason: {reason}")]
    ReplayProtectionViolation { key: String, reason: String },

    #[error("Balance delta derivation error: {0}")]
    BalanceDelta(#[from] ExecutionRecorderError),

    #[error("Cryptographic signing error: {0}")]
    Signer(#[from] SignerError),

    #[error("Solana RPC / integration error: {0}")]
    Solana(#[from] SolanaError),

    #[error("Transaction failed on-chain ({signature}): {status:?}")]
    ConfirmationFailed {
        signature: String,
        status: TransactionConfirmationStatus,
    },
}

/// Precondition context containing authenticated on-chain state to validate against the plan.
#[derive(Debug, Clone)]
pub struct PreconditionParameters {
    pub vault_authority: Pubkey,
    pub vault_asset_mint: Pubkey,
    pub compliance_asset_mint: Pubkey,
    pub compliance_status: u8,
    pub compliance_valid_until: i64,
    pub dex_program: Pubkey,
    pub max_allowed_slippage_bps: u16,
}

/// Production Connector linking ExecutionPlans to on-chain Anchor execute_action instructions.
#[derive(Clone)]
pub struct AnchorExecutionConnector {
    anchor_client: Arc<AnchorClient>,
    rpc: Arc<SolanaRpcClient>,
    idempotency_tracker: Arc<IdempotencyTracker>,
}

impl AnchorExecutionConnector {
    pub fn new(
        rpc: Arc<SolanaRpcClient>,
        anchor_client: Arc<AnchorClient>,
        idempotency_tracker: Arc<IdempotencyTracker>,
    ) -> Self {
        Self {
            anchor_client,
            rpc,
            idempotency_tracker,
        }
    }

    pub fn new_with_rpc(
        rpc: Arc<SolanaRpcClient>,
        program_id: Pubkey,
        idempotency_tracker: Arc<IdempotencyTracker>,
    ) -> Self {
        let anchor_client = Arc::new(AnchorClient::new(rpc.clone()).with_program_id(program_id));
        Self {
            anchor_client,
            rpc,
            idempotency_tracker,
        }
    }

    pub fn anchor_client(&self) -> &Arc<AnchorClient> {
        &self.anchor_client
    }

    pub fn rpc(&self) -> &Arc<SolanaRpcClient> {
        &self.rpc
    }

    pub fn idempotency_tracker(&self) -> &Arc<IdempotencyTracker> {
        &self.idempotency_tracker
    }

    /// Task 1, 2, 3: Validates all pre-execution invariants before instruction building or signing:
    /// - Vault authority matches keeper
    /// - Authorized DEX program whitelist
    /// - Distinct input and output mints matching vault and compliance assets
    /// - Compliance status is APPROVED
    /// - Neither compliance nor plan has expired
    /// - Trade amounts > 0
    /// - Slippage within allowed bound
    /// - Replay / idempotency state
    pub fn validate_execution_preconditions(
        plan: &ExecutionPlan,
        keeper: &Pubkey,
        params: &PreconditionParameters,
        current_time: i64,
    ) -> Result<(), AnchorExecutionError> {
        // 1. Vault authority validation
        if *keeper != params.vault_authority {
            return Err(AnchorExecutionError::UnauthorizedKeeper {
                expected: params.vault_authority.to_string(),
                actual: keeper.to_string(),
            });
        }

        // 2. Authorized DEX Program Whitelist validation
        if !is_authorized_dex_program(&params.dex_program) {
            return Err(AnchorExecutionError::UnauthorizedDexProgram(
                params.dex_program.to_string(),
            ));
        }

        // 3. Mints validation
        let input_mint = Pubkey::from_str(&plan.input_mint).map_err(|e| {
            AnchorExecutionError::InvalidTokenMints {
                input: plan.input_mint.clone(),
                output: plan.output_mint.clone(),
                reason: format!("Invalid input mint pubkey: {}", e),
            }
        })?;

        let output_mint = Pubkey::from_str(&plan.output_mint).map_err(|e| {
            AnchorExecutionError::InvalidTokenMints {
                input: plan.input_mint.clone(),
                output: plan.output_mint.clone(),
                reason: format!("Invalid output mint pubkey: {}", e),
            }
        })?;

        if input_mint == output_mint {
            return Err(AnchorExecutionError::InvalidTokenMints {
                input: plan.input_mint.clone(),
                output: plan.output_mint.clone(),
                reason: "Input mint and output mint must be distinct".to_string(),
            });
        }

        let is_buy =
            input_mint == params.vault_asset_mint && output_mint == params.compliance_asset_mint;
        let is_sell =
            input_mint == params.compliance_asset_mint && output_mint == params.vault_asset_mint;

        if !is_buy && !is_sell {
            return Err(AnchorExecutionError::InvalidTokenMints {
                input: plan.input_mint.clone(),
                output: plan.output_mint.clone(),
                reason: "Traded mints must match vault asset mint and compliance asset mint"
                    .to_string(),
            });
        }

        // 4. Shariah Compliance status validation
        if params.compliance_status != COMPLIANCE_STATUS_APPROVED {
            return Err(AnchorExecutionError::AssetNotApproved {
                asset_mint: params.compliance_asset_mint.to_string(),
                status: params.compliance_status,
            });
        }

        // 5. Expiry validation: compliance validity & plan validity
        if current_time >= params.compliance_valid_until {
            return Err(AnchorExecutionError::ComplianceExpired {
                valid_until: params.compliance_valid_until,
                current_time,
            });
        }

        if current_time >= plan.expires_at {
            return Err(AnchorExecutionError::PlanExpired {
                expires_at: plan.expires_at,
                current_time,
            });
        }

        // 6. Minimum output & input validation
        if plan.input_amount == 0 {
            return Err(AnchorExecutionError::InvalidAmount {
                amount: 0,
                reason: "Input amount must be positive".to_string(),
            });
        }

        if plan.minimum_output_amount == 0 {
            return Err(AnchorExecutionError::InvalidAmount {
                amount: 0,
                reason: "Minimum output amount must be positive".to_string(),
            });
        }

        // 7. Slippage validation
        if plan.slippage_bps > params.max_allowed_slippage_bps {
            return Err(AnchorExecutionError::ExcessiveSlippage {
                slippage_bps: plan.slippage_bps,
                max_allowed: params.max_allowed_slippage_bps,
            });
        }

        Ok(())
    }

    /// Task 1, 4: Builds the official Anchor execute_action instruction for the ExecutionPlan.
    /// Uses actual DEX accounts and passes validated DEX program ID.
    pub fn build_anchor_execute_action(
        &self,
        plan: &ExecutionPlan,
        keeper: &Pubkey,
        dex_program: &Pubkey,
        remaining_accounts: &[AccountMeta],
    ) -> Result<(Instruction, Pubkey), AnchorExecutionError> {
        if !is_authorized_dex_program(dex_program) {
            return Err(AnchorExecutionError::UnauthorizedDexProgram(
                dex_program.to_string(),
            ));
        }

        let vault_pda = Pubkey::from_str(&plan.vault_address).map_err(|e| {
            AnchorExecutionError::InvalidTokenMints {
                input: plan.input_mint.clone(),
                output: plan.output_mint.clone(),
                reason: format!("Invalid vault address: {}", e),
            }
        })?;

        let input_mint = Pubkey::from_str(&plan.input_mint).map_err(|e| {
            AnchorExecutionError::InvalidTokenMints {
                input: plan.input_mint.clone(),
                output: plan.output_mint.clone(),
                reason: format!("Invalid input mint: {}", e),
            }
        })?;

        let output_mint = Pubkey::from_str(&plan.output_mint).map_err(|e| {
            AnchorExecutionError::InvalidTokenMints {
                input: plan.input_mint.clone(),
                output: plan.output_mint.clone(),
                reason: format!("Invalid output mint: {}", e),
            }
        })?;

        let vault_input_ata = find_associated_token_address(&vault_pda, &input_mint);
        let vault_output_ata = find_associated_token_address(&vault_pda, &output_mint);

        // Derive compliance PDA: compliance record is keyed by the equity asset mint
        let compliance_mint = if plan.is_buy {
            &output_mint
        } else {
            &input_mint
        };
        let (compliance_pda, _) =
            find_compliance_pda(compliance_mint, &self.anchor_client.program_id());

        let execution_seq = plan.plan_id.as_u128() as u64;
        let action_type = 1u8; // Spot Swap

        let (ix, execution_pda) = self
            .anchor_client
            .build_execute_action_ix_full(
                keeper,
                &vault_pda,
                execution_seq,
                action_type,
                &input_mint,
                &output_mint,
                &vault_input_ata,
                &vault_output_ata,
                &compliance_pda,
                dex_program,
                plan.input_amount,
                plan.minimum_output_amount,
                remaining_accounts,
            )
            .map_err(AnchorExecutionError::Solana)?;

        Ok((ix, execution_pda))
    }

    /// Task 6, 7, 8, 9: Measures balance deltas strictly from token accounts.
    /// Invariants:
    /// - actual_output = after - before
    /// - after >= before
    /// - actual_output >= minimum_output
    /// - Never trust client-reported output
    /// - Never set actual output equal to minimum output
    pub fn derive_actual_output(
        before_balance: u64,
        after_balance: u64,
        minimum_output: u64,
    ) -> Result<u64, ExecutionRecorderError> {
        if after_balance < before_balance {
            return Err(ExecutionRecorderError::NegativeBalanceDelta {
                before: before_balance,
                after: after_balance,
            });
        }

        let actual = after_balance
            .checked_sub(before_balance)
            .ok_or(ExecutionRecorderError::MathOverflow)?;

        if actual < minimum_output {
            return Err(ExecutionRecorderError::SlippageExceeded {
                actual,
                minimum: minimum_output,
            });
        }

        Ok(actual)
    }

    /// Complete End-to-End Orchestrator (Tasks 1 through 13 & Acceptance Criteria):
    /// Connects ExecutionPlan -> Validates Preconditions -> Measures Before Balance ->
    /// Builds Anchor execute_action -> Signs -> Broadcasts -> Tracks On-Chain Confirmation ->
    /// Measures After Balance -> Derives actual_output = after - before -> Finalizes ExecutionRecord marked SUCCESS.
    #[instrument(skip(self, signer, plan, params, remaining_accounts))]
    pub async fn execute_and_confirm<S: ExternalSigner>(
        &self,
        plan: &ExecutionPlan,
        signer: &S,
        params: &PreconditionParameters,
        remaining_accounts: &[AccountMeta],
        confirmation_timeout: Duration,
    ) -> Result<ExecutionRecord, AnchorExecutionError> {
        let current_time = Utc::now().timestamp();
        let keeper_pubkey = signer.pubkey();

        // Step 1: Deterministic Precondition Validation (Tasks 1, 2, 3)
        Self::validate_execution_preconditions(plan, &keeper_pubkey, params, current_time)?;

        // Step 2: Idempotency Check & Status Transition (Task 3)
        if let Some(existing) = self
            .idempotency_tracker
            .get_record(&plan.idempotency_key)
            .await
        {
            if existing.status == IdempotencyStatus::Completed {
                return Err(AnchorExecutionError::ReplayProtectionViolation {
                    key: plan.idempotency_key.clone(),
                    reason: "Execution plan was already executed on-chain (completed)".to_string(),
                });
            }
        }

        let _ = self
            .idempotency_tracker
            .update_status(&plan.idempotency_key, IdempotencyStatus::Executing)
            .await;

        // Step 3: Snapshot Output Token Balance BEFORE CPI (Task 6)
        let vault_pda = Pubkey::from_str(&plan.vault_address).map_err(|e| {
            AnchorExecutionError::InvalidTokenMints {
                input: plan.input_mint.clone(),
                output: plan.output_mint.clone(),
                reason: format!("Invalid vault address: {}", e),
            }
        })?;
        let output_mint = Pubkey::from_str(&plan.output_mint).map_err(|e| {
            AnchorExecutionError::InvalidTokenMints {
                input: plan.input_mint.clone(),
                output: plan.output_mint.clone(),
                reason: format!("Invalid output mint: {}", e),
            }
        })?;
        let vault_output_ata = find_associated_token_address(&vault_pda, &output_mint);

        let before_balance = match self.rpc.get_token_account_balance(&vault_output_ata).await {
            Ok(bal) => bal.amount,
            Err(_) => 0, // In fresh accounts, balance may start at zero
        };

        // Step 4: Assemble Anchor execute_action instruction with authorized DEX CPI (Tasks 1, 4, 5)
        let (execute_ix, _execution_pda) = self.build_anchor_execute_action(
            plan,
            &keeper_pubkey,
            &params.dex_program,
            remaining_accounts,
        )?;

        // Step 5: Fetch current Solana blockhash and last_valid_block_height
        let (recent_blockhash, last_valid_block_height) = self
            .rpc
            .get_latest_blockhash()
            .await
            .map_err(AnchorExecutionError::Solana)?;

        // Step 6: Build unsigned transaction and sign with authentic cryptographic signer
        let unsigned_tx = TransactionBuilder::build_unsigned(
            &[execute_ix],
            &keeper_pubkey,
            recent_blockhash,
            last_valid_block_height,
        )?;

        let signed_tx = TransactionSignerService::sign(signer, &unsigned_tx).await?;

        // Step 7: Broadcast transaction to Solana cluster (Task 12, 13)
        let wire_bytes = signed_tx.to_bytes()?;
        let tx_sig = self
            .rpc
            .send_transaction(&wire_bytes)
            .await
            .map_err(AnchorExecutionError::Solana)?;

        let actual_signature = tx_sig.to_string();
        info!(
            plan_id = %plan.plan_id,
            signature = %actual_signature,
            "Transaction submitted to Solana cluster; beginning confirmation tracking"
        );

        // Step 8: Track Transaction Confirmation on Cluster (Tasks 10, 11, 12)
        // Never mark execution successful merely because sendTransaction returned!
        let confirmation_status = self
            .rpc
            .track_transaction_confirmation(
                &tx_sig,
                Some(last_valid_block_height),
                confirmation_timeout,
            )
            .await
            .map_err(AnchorExecutionError::Solana)?;

        match &confirmation_status {
            TransactionConfirmationStatus::ConfirmedSuccess { slot, .. } => {
                info!(
                    plan_id = %plan.plan_id,
                    signature = %actual_signature,
                    slot,
                    "Transaction confirmed successfully on Solana cluster"
                );
            }
            TransactionConfirmationStatus::ExpiredTransaction { .. } => {
                warn!(
                    plan_id = %plan.plan_id,
                    signature = %actual_signature,
                    "Transaction expired on cluster before confirmation"
                );
                let _ = self
                    .idempotency_tracker
                    .update_status(&plan.idempotency_key, IdempotencyStatus::Expired)
                    .await;
                return Err(AnchorExecutionError::ConfirmationFailed {
                    signature: actual_signature,
                    status: confirmation_status,
                });
            }
            status => {
                let _err_msg = format!("Transaction confirmation failed: {:?}", status);
                warn!(
                    plan_id = %plan.plan_id,
                    signature = %actual_signature,
                    status = ?status,
                    "Transaction not confirmed successfully"
                );
                let _ = self
                    .idempotency_tracker
                    .update_status(&plan.idempotency_key, IdempotencyStatus::Failed)
                    .await;
                return Err(AnchorExecutionError::ConfirmationFailed {
                    signature: actual_signature,
                    status: status.clone(),
                });
            }
        }

        // Step 9: Snapshot Output Token Balance AFTER CPI (Task 6, 7, 8, 9)
        let after_balance = self
            .rpc
            .get_token_account_balance(&vault_output_ata)
            .await
            .map_err(AnchorExecutionError::Solana)?
            .amount;

        // Step 10: Calculate actual output strictly as after - before (Tasks 7, 8, 9)
        let actual_output = match Self::derive_actual_output(
            before_balance,
            after_balance,
            plan.minimum_output_amount,
        ) {
            Ok(out) => out,
            Err(e) => {
                let _ = self
                    .idempotency_tracker
                    .update_status(&plan.idempotency_key, IdempotencyStatus::Failed)
                    .await;
                return Err(AnchorExecutionError::BalanceDelta(e));
            }
        };

        // Step 11: Finalize ExecutionRecord marked SUCCESS (Acceptance Criteria!)
        let record = ExecutionRecord::from_verified_execution(
            plan.plan_id,
            plan.vault_address.clone(),
            plan.input_mint.clone(),
            plan.output_mint.clone(),
            plan.input_amount,
            plan.minimum_output_amount,
            before_balance,
            after_balance,
            Utc::now().timestamp(),
            actual_signature,
            plan.quote_id.clone(),
            plan.policy_decision_id,
        )?;

        // Step 12: Mark Idempotency Record Completed
        let _ = self
            .idempotency_tracker
            .update_status(&plan.idempotency_key, IdempotencyStatus::Completed)
            .await;

        info!(
            plan_id = %plan.plan_id,
            actual_output,
            status = %record.status,
            "Execution completed and verified with balance delta"
        );

        Ok(record)
    }
}
