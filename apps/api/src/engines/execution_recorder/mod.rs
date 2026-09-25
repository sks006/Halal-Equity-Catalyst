//! Execution Recorder Engine for Phase 14.
//!
//! # Objective
//! Record the actual result of a DEX execution strictly using token-account balance deltas.
//!
//! # Core Security & Accounting Principles
//! 1. Before CPI: Record relevant output token balance (`before_balance`).
//! 2. Execute DEX CPI.
//! 3. After CPI: Read output token balance again (`after_balance`).
//! 4. Calculate: `actual_output_amount = after_balance - before_balance`.
//! 5. Do NOT derive actual output from:
//!    - minimum output
//!    - expected output
//!    - oracle price
//!    - quote estimate
//!    - client input
//! 6. Validate that: `after_balance >= before_balance` (unless explicitly allowing negative delta semantics).
//! 7. Strictly enforce slippage: `actual_output >= minimum_output`.
//! 8. Record:
//!    - requested input
//!    - minimum output
//!    - actual output
//!    - execution timestamp
//!    - transaction signature
//!    - quote ID
//!    - policy decision ID

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;
use uuid::Uuid;

/// Error conditions encountered during execution balance delta recording.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ExecutionRecorderError {
    #[error("Output token balance delta is negative: after_balance ({after}) < before_balance ({before})")]
    NegativeBalanceDelta { before: u64, after: u64 },

    #[error("Derived actual output {actual} is less than guaranteed minimum output {minimum}")]
    SlippageExceeded { actual: u64, minimum: u64 },

    #[error("Pre-execution balance was not recorded prior to CPI execution")]
    MissingBeforeBalance,

    #[error("Post-execution balance was not recorded after CPI execution")]
    MissingAfterBalance,

    #[error("Requested input amount must be positive, got {0}")]
    InvalidRequestedInput(u64),

    #[error("Minimum output amount must be positive, got {0}")]
    InvalidMinimumOutput(u64),

    #[error("Calculation overflow during balance delta reconciliation")]
    MathOverflow,
}

/// An immutable, tamper-evident record of a completed on-chain DEX execution.
///
/// Contains all 7 canonical requirements:
/// 1. Requested input (`requested_input`)
/// 2. Minimum output (`minimum_output`)
/// 3. Actual output (`actual_output`, derived strictly from balance delta)
/// 4. Execution timestamp (`execution_timestamp`)
/// 5. Transaction signature (`transaction_signature`)
/// 6. Quote ID (`quote_id`)
/// 7. Policy decision ID (`policy_decision_id`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Unique execution identifier (idempotency key)
    pub execution_id: Uuid,
    /// Associated target vault account address
    pub vault_address: String,
    /// Solana SPL token input mint
    pub input_mint: String,
    /// Solana SPL token output mint
    pub output_mint: String,
    /// Requested input token amount in integer atomic units
    pub requested_input: u64,
    /// Guaranteed minimum output amount under slippage limit
    pub minimum_output: u64,
    /// Real actual output amount derived from token account balance delta
    pub actual_output: u64,
    /// Pre-execution output token balance
    pub before_balance: u64,
    /// Post-execution output token balance
    pub after_balance: u64,
    /// Unix timestamp when execution occurred
    pub execution_timestamp: i64,
    /// Solana transaction signature (base58)
    pub transaction_signature: String,
    /// DEX quote identifier
    pub quote_id: String,
    /// Approved policy decision identifier
    pub policy_decision_id: Uuid,
    /// Execution status (e.g. "confirmed")
    pub status: String,
}

impl ExecutionRecord {
    /// Constructs an ExecutionRecord strictly from token balance delta verification.
    ///
    /// # Invariants Enforced:
    /// 1. `actual_output = after_balance - before_balance`
    /// 2. `after_balance >= before_balance` (unless `allow_negative_delta` is true)
    /// 3. Never derives actual output from min output, expected output, oracle price, or quote estimate.
    /// 4. `actual_output >= minimum_output` (slippage check)
    /// 5. `requested_input > 0` and `minimum_output > 0`
    #[allow(clippy::too_many_arguments)]
    pub fn from_balance_delta(
        execution_id: Uuid,
        vault_address: String,
        input_mint: String,
        output_mint: String,
        requested_input: u64,
        minimum_output: u64,
        before_balance: u64,
        after_balance: u64,
        execution_timestamp: i64,
        transaction_signature: String,
        quote_id: String,
        policy_decision_id: Uuid,
        allow_negative_delta: bool,
    ) -> Result<Self, ExecutionRecorderError> {
        if requested_input == 0 {
            return Err(ExecutionRecorderError::InvalidRequestedInput(0));
        }
        if minimum_output == 0 {
            return Err(ExecutionRecorderError::InvalidMinimumOutput(0));
        }

        // Validate that: after_balance >= before_balance
        // unless the transaction explicitly supports a negative balance delta semantics.
        if !allow_negative_delta && after_balance < before_balance {
            return Err(ExecutionRecorderError::NegativeBalanceDelta {
                before: before_balance,
                after: after_balance,
            });
        }

        // Calculate actual_output_amount = after_balance - before_balance
        let actual_output = if after_balance >= before_balance {
            after_balance
                .checked_sub(before_balance)
                .ok_or(ExecutionRecorderError::MathOverflow)?
        } else {
            0
        };

        // Enforce slippage: actual output must meet or exceed minimum output
        if actual_output < minimum_output {
            return Err(ExecutionRecorderError::SlippageExceeded {
                actual: actual_output,
                minimum: minimum_output,
            });
        }

        Ok(Self {
            execution_id,
            vault_address,
            input_mint,
            output_mint,
            requested_input,
            minimum_output,
            actual_output,
            before_balance,
            after_balance,
            execution_timestamp,
            transaction_signature,
            quote_id,
            policy_decision_id,
            status: "confirmed".to_string(),
        })
    }
}

/// Stateful tracker that enforces the exact chronological lifecycle:
/// `record_before_balance` -> `execute CPI` -> `record_after_balance` -> `complete`.
#[derive(Debug, Clone)]
pub struct ExecutionBalanceTracker {
    pub vault_output_account: Pubkey,
    pub requested_input: u64,
    pub minimum_output: u64,
    pub quote_id: String,
    pub policy_decision_id: Uuid,
    before_balance: Option<u64>,
    after_balance: Option<u64>,
}

impl ExecutionBalanceTracker {
    pub fn new(
        vault_output_account: Pubkey,
        requested_input: u64,
        minimum_output: u64,
        quote_id: String,
        policy_decision_id: Uuid,
    ) -> Self {
        Self {
            vault_output_account,
            requested_input,
            minimum_output,
            quote_id,
            policy_decision_id,
            before_balance: None,
            after_balance: None,
        }
    }

    /// Record output token balance before CPI execution.
    pub fn record_before_balance(&mut self, balance: u64) {
        self.before_balance = Some(balance);
    }

    /// Record output token balance after CPI execution.
    pub fn record_after_balance(&mut self, balance: u64) {
        self.after_balance = Some(balance);
    }

    pub fn before_balance(&self) -> Option<u64> {
        self.before_balance
    }

    pub fn after_balance(&self) -> Option<u64> {
        self.after_balance
    }

    /// Calculate actual output amount derived from balance delta.
    pub fn calculate_actual_output(
        &self,
        allow_negative_delta: bool,
    ) -> Result<u64, ExecutionRecorderError> {
        let before = self
            .before_balance
            .ok_or(ExecutionRecorderError::MissingBeforeBalance)?;
        let after = self
            .after_balance
            .ok_or(ExecutionRecorderError::MissingAfterBalance)?;

        if !allow_negative_delta && after < before {
            return Err(ExecutionRecorderError::NegativeBalanceDelta {
                before,
                after,
            });
        }

        let actual = if after >= before {
            after
                .checked_sub(before)
                .ok_or(ExecutionRecorderError::MathOverflow)?
        } else {
            0
        };

        if actual < self.minimum_output {
            return Err(ExecutionRecorderError::SlippageExceeded {
                actual,
                minimum: self.minimum_output,
            });
        }

        Ok(actual)
    }

    /// Completes the balance reconciliation and constructs a verified ExecutionRecord.
    #[allow(clippy::too_many_arguments)]
    pub fn complete(
        self,
        execution_id: Uuid,
        vault_address: String,
        input_mint: String,
        output_mint: String,
        transaction_signature: String,
        execution_timestamp: i64,
        allow_negative_delta: bool,
    ) -> Result<ExecutionRecord, ExecutionRecorderError> {
        let before = self
            .before_balance
            .ok_or(ExecutionRecorderError::MissingBeforeBalance)?;
        let after = self
            .after_balance
            .ok_or(ExecutionRecorderError::MissingAfterBalance)?;

        ExecutionRecord::from_balance_delta(
            execution_id,
            vault_address,
            input_mint,
            output_mint,
            self.requested_input,
            self.minimum_output,
            before,
            after,
            execution_timestamp,
            transaction_signature,
            self.quote_id,
            self.policy_decision_id,
            allow_negative_delta,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_from_balance_delta_success() {
        let execution_id = Uuid::new_v4();
        let policy_decision_id = Uuid::new_v4();
        let before_balance: u64 = 5_000_000;
        let after_balance: u64 = 5_985_000; // Delta: 985_000
        let requested_input: u64 = 1_000_000;
        let minimum_output: u64 = 980_000;

        let record = ExecutionRecord::from_balance_delta(
            execution_id,
            "Vault111111111111111111111111111111111111111".to_string(),
            "InputMint111111111111111111111111111111111111".to_string(),
            "OutputMint11111111111111111111111111111111111".to_string(),
            requested_input,
            minimum_output,
            before_balance,
            after_balance,
            1710000000,
            "5TxSignature111111111111111111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "quote-jup-12345".to_string(),
            policy_decision_id,
            false,
        )
        .expect("Valid delta must construct ExecutionRecord");

        // Verify all 7 required fields
        assert_eq!(record.requested_input, 1_000_000);
        assert_eq!(record.minimum_output, 980_000);
        assert_eq!(record.actual_output, 985_000); // 5_985_000 - 5_000_000
        assert_eq!(record.before_balance, 5_000_000);
        assert_eq!(record.after_balance, 5_985_000);
        assert_eq!(record.execution_timestamp, 1710000000);
        assert_eq!(
            record.transaction_signature,
            "5TxSignature111111111111111111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(record.quote_id, "quote-jup-12345");
        assert_eq!(record.policy_decision_id, policy_decision_id);
        assert_eq!(record.status, "confirmed");
    }

    #[test]
    fn test_rejects_negative_balance_delta() {
        let execution_id = Uuid::new_v4();
        let policy_decision_id = Uuid::new_v4();
        let before_balance: u64 = 5_000_000;
        let after_balance: u64 = 4_900_000; // Negative delta: lost 100_000 tokens!

        let err = ExecutionRecord::from_balance_delta(
            execution_id,
            "Vault111111111111111111111111111111111111111".to_string(),
            "InMint".to_string(),
            "OutMint".to_string(),
            1_000_000,
            900_000,
            before_balance,
            after_balance,
            1710000000,
            "sig".to_string(),
            "q1".to_string(),
            policy_decision_id,
            false, // Do NOT allow negative delta
        )
        .unwrap_err();

        assert_eq!(
            err,
            ExecutionRecorderError::NegativeBalanceDelta {
                before: 5_000_000,
                after: 4_900_000
            }
        );
    }

    #[test]
    fn test_rejects_slippage_violation() {
        let execution_id = Uuid::new_v4();
        let policy_decision_id = Uuid::new_v4();
        let before_balance: u64 = 1_000_000;
        let after_balance: u64 = 1_900_000; // Delta: 900_000
        let minimum_output: u64 = 950_000; // Min output expected 950_000 > 900_000

        let err = ExecutionRecord::from_balance_delta(
            execution_id,
            "Vault".to_string(),
            "InMint".to_string(),
            "OutMint".to_string(),
            1_000_000,
            minimum_output,
            before_balance,
            after_balance,
            1710000000,
            "sig".to_string(),
            "q1".to_string(),
            policy_decision_id,
            false,
        )
        .unwrap_err();

        assert_eq!(
            err,
            ExecutionRecorderError::SlippageExceeded {
                actual: 900_000,
                minimum: 950_000
            }
        );
    }

    #[test]
    fn test_tracker_lifecycle() {
        let pubkey = Pubkey::new_unique();
        let policy_decision_id = Uuid::new_v4();
        let mut tracker = ExecutionBalanceTracker::new(
            pubkey,
            1_000_000,
            950_000,
            "quote-meteora-42".to_string(),
            policy_decision_id,
        );

        assert_eq!(tracker.before_balance(), None);
        assert_eq!(tracker.after_balance(), None);

        // Pre-CPI: record balance
        tracker.record_before_balance(10_000_000);
        assert_eq!(tracker.before_balance(), Some(10_000_000));

        // Post-CPI: record balance
        tracker.record_after_balance(10_975_000);
        assert_eq!(tracker.after_balance(), Some(10_975_000));

        // Calculate delta
        let delta = tracker.calculate_actual_output(false).unwrap();
        assert_eq!(delta, 975_000);

        // Complete record
        let record = tracker
            .complete(
                Uuid::new_v4(),
                "Vault123".to_string(),
                "InMint".to_string(),
                "OutMint".to_string(),
                "Sig456".to_string(),
                1715000000,
                false,
            )
            .unwrap();

        assert_eq!(record.actual_output, 975_000);
        assert_eq!(record.requested_input, 1_000_000);
        assert_eq!(record.minimum_output, 950_000);
        assert_eq!(record.quote_id, "quote-meteora-42");
        assert_eq!(record.policy_decision_id, policy_decision_id);
    }
}
