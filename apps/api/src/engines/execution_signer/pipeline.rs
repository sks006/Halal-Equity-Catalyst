//! Strict separation of the transaction execution lifecycle:
//! 1. Transaction Construction (pure instruction and message building)
//! 2. Transaction Signing (cryptographic signing via ExternalSigner)
//! 3. Transaction Submission (broadcasting signed payload to Solana cluster)
//!
//! # Core Security Principle
//! Each stage is strictly isolated:
//! - Construction stage CANNOT sign or submit.
//! - Signing stage CANNOT submit.
//! - Submission stage CANNOT sign or construct.

use equity_catalyst_solana::rpc::SolanaRpcClient;
use solana_sdk::{
    hash::Hash, instruction::Instruction, message::Message, pubkey::Pubkey, signature::Signature,
    transaction::Transaction,
};
use tracing::{info, instrument};

use super::error::SignerError;
use super::external::ExternalSigner;
use crate::engines::execution_planner::ExecutionPlan;

// ============================================================================
// STAGE 1: TRANSACTION CONSTRUCTION
// ============================================================================

/// Canonical placeholder program ID constant for testing/stubs.
pub const PLACEHOLDER_PROGRAM_ID: &str = "H5hM4fqRjygvCYXnp6dgFLgZ6o4uJ8Q9z7dAsTfapHmF";
pub const PLACEHOLDER_PROGRAM_BYTES: [u8; 32] = [0xEE; 32];

/// Checks whether an instruction is an unexecutable placeholder, mock, or stub instruction.
pub fn is_placeholder_instruction(ix: &Instruction) -> bool {
    if ix.program_id == Pubkey::new_from_array(PLACEHOLDER_PROGRAM_BYTES) {
        return true;
    }
    let prog_str = ix.program_id.to_string();
    if prog_str == PLACEHOLDER_PROGRAM_ID
        || prog_str.starts_with("Placeholder")
        || prog_str.to_lowercase().contains("placeholder")
        || prog_str.to_lowercase().contains("mock_program")
    {
        return true;
    }

    if ix.accounts.is_empty() && ix.data.is_empty() {
        return true;
    }

    let lower_data = ix.data.to_ascii_lowercase();
    if lower_data == b"placeholder"
        || lower_data.starts_with(b"placeholder_")
        || lower_data == b"dummy"
        || lower_data == b"mock"
    {
        return true;
    }

    false
}

/// An un-signed Solana transaction with explicit message and instructions.
#[derive(Debug, Clone, PartialEq)]
pub struct UnsignedTransaction {
    pub transaction: Transaction,
    pub payer: Pubkey,
    pub instructions: Vec<Instruction>,
    pub recent_blockhash: Hash,
    pub last_valid_block_height: u64,
}

/// Pure transaction construction engine.
///
/// Builds instruction messages and transaction framing without signing or submitting.
pub struct TransactionBuilder;

impl TransactionBuilder {
    /// Constructs an unsigned transaction from raw instructions, fee payer, recent blockhash, and last valid block height.
    ///
    /// Rejects empty instructions, zero blockhash, or placeholder instructions.
    pub fn build_unsigned(
        instructions: &[Instruction],
        payer: &Pubkey,
        recent_blockhash: Hash,
        last_valid_block_height: u64,
    ) -> Result<UnsignedTransaction, SignerError> {
        if instructions.is_empty() {
            return Err(SignerError::InvalidTransactionState(
                "Cannot build transaction with empty instructions".to_string(),
            ));
        }
        if recent_blockhash == Hash::default() {
            return Err(SignerError::InvalidTransactionState(
                "Cannot build transaction with default/zero blockhash".to_string(),
            ));
        }

        // Deterministic check: Never construct or sign a placeholder instruction
        for ix in instructions {
            if is_placeholder_instruction(ix) {
                return Err(SignerError::PlaceholderInstructionProhibited {
                    program: ix.program_id.to_string(),
                });
            }
        }

        let message = Message::new(instructions, Some(payer));
        let transaction = Transaction::new_unsigned(message);

        Ok(UnsignedTransaction {
            transaction,
            payer: *payer,
            instructions: instructions.to_vec(),
            recent_blockhash,
            last_valid_block_height,
        })
    }

    /// Constructs an unsigned transaction without explicit block height (defaults height to 0).
    pub fn build_unsigned_legacy(
        instructions: &[Instruction],
        payer: &Pubkey,
        recent_blockhash: Hash,
    ) -> Result<UnsignedTransaction, SignerError> {
        Self::build_unsigned(instructions, payer, recent_blockhash, 0)
    }

    /// Constructs an unsigned transaction representing an approved ExecutionPlan trade.
    pub fn build_from_plan(
        plan: &ExecutionPlan,
        payer: &Pubkey,
        recent_blockhash: Hash,
        custom_instructions: &[Instruction],
    ) -> Result<UnsignedTransaction, SignerError> {
        Self::build_from_plan_with_block_height(
            plan,
            payer,
            recent_blockhash,
            0,
            custom_instructions,
        )
    }

    /// Constructs an unsigned transaction from an approved ExecutionPlan with block height tracking.
    pub fn build_from_plan_with_block_height(
        plan: &ExecutionPlan,
        payer: &Pubkey,
        recent_blockhash: Hash,
        last_valid_block_height: u64,
        custom_instructions: &[Instruction],
    ) -> Result<UnsignedTransaction, SignerError> {
        if custom_instructions.is_empty() {
            let memo_ix = Instruction::new_with_bytes(
                solana_sdk::pubkey::Pubkey::new_unique(),
                &plan.to_canonical_bytes(),
                vec![],
            );
            Self::build_unsigned(&[memo_ix], payer, recent_blockhash, last_valid_block_height)
        } else {
            Self::build_unsigned(
                custom_instructions,
                payer,
                recent_blockhash,
                last_valid_block_height,
            )
        }
    }
}

// ============================================================================
// STAGE 2: TRANSACTION SIGNING
// ============================================================================

/// A cryptographically signed Solana transaction ready for transmission.
#[derive(Debug, Clone, PartialEq)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub primary_signature: Signature,
    pub signer_pubkey: Pubkey,
    pub signer_type: &'static str,
    pub recent_blockhash: Hash,
    pub last_valid_block_height: u64,
}

impl SignedTransaction {
    /// Returns the serialized transaction wire bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SignerError> {
        bincode::serialize(&self.transaction).map_err(|e| {
            SignerError::SigningFailed(format!("Failed to serialize signed transaction: {}", e))
        })
    }
}

/// Cryptographic signing service isolated from construction and network submission.
pub struct TransactionSignerService;

impl TransactionSignerService {
    /// Applies real cryptographic signatures to an unsigned transaction using an ExternalSigner.
    ///
    /// Validates all deterministic checks prior to signing:
    /// - Signer is available and reachable
    /// - Signer public key matches the transaction fee payer
    /// - No placeholder instructions are present in the transaction
    /// - Transaction recent blockhash is non-zero
    #[instrument(skip(signer, unsigned))]
    pub async fn sign(
        signer: &dyn ExternalSigner,
        unsigned: &UnsignedTransaction,
    ) -> Result<SignedTransaction, SignerError> {
        // Enforce fail-closed check if the external signer is unavailable
        if !signer.is_available() {
            return Err(SignerError::SignerUnavailable {
                signer_type: signer.signer_type(),
                reason: "Signer is offline or disconnected; failing closed".to_string(),
            });
        }

        // Verify payer matches signer pubkey
        if signer.pubkey() != unsigned.payer {
            return Err(SignerError::AuthorityMismatch {
                expected: unsigned.payer.to_string(),
                actual: signer.pubkey_string(),
            });
        }

        // Deterministic check: Never sign a placeholder instruction
        for ix in &unsigned.instructions {
            if is_placeholder_instruction(ix) {
                return Err(SignerError::PlaceholderInstructionProhibited {
                    program: ix.program_id.to_string(),
                });
            }
        }

        if unsigned.recent_blockhash == Hash::default() {
            return Err(SignerError::InvalidTransactionState(
                "Transaction blockhash must be set before signing".to_string(),
            ));
        }

        let mut tx = unsigned.transaction.clone();
        tx.message.recent_blockhash = unsigned.recent_blockhash;

        signer.sign_transaction(&mut tx)?;

        let primary_signature = tx.signatures.first().copied().ok_or_else(|| {
            SignerError::SigningFailed("No signature was produced on the transaction".to_string())
        })?;

        // Verify the primary signature is not a default/empty signature
        if primary_signature == Signature::default() {
            return Err(SignerError::SigningFailed(
                "Produced signature is default/empty; signing aborted".to_string(),
            ));
        }

        info!(
            signer_type = signer.signer_type(),
            signer_pubkey = %signer.pubkey(),
            signature = %primary_signature,
            recent_blockhash = %unsigned.recent_blockhash,
            last_valid_block_height = unsigned.last_valid_block_height,
            "Transaction cryptographically signed with authentic signature"
        );

        Ok(SignedTransaction {
            transaction: tx,
            primary_signature,
            signer_pubkey: signer.pubkey(),
            signer_type: signer.signer_type(),
            recent_blockhash: unsigned.recent_blockhash,
            last_valid_block_height: unsigned.last_valid_block_height,
        })
    }
}

// ============================================================================
// STAGE 3: TRANSACTION SUBMISSION
// ============================================================================

/// Network submission engine isolated from key material and construction.
pub struct TransactionSubmitter;

impl TransactionSubmitter {
    /// Submits a pre-signed transaction to the Solana cluster via RPC.
    #[instrument(skip(rpc, signed))]
    pub async fn submit(
        rpc: &SolanaRpcClient,
        signed: &SignedTransaction,
    ) -> Result<Signature, SignerError> {
        // Verify transaction is validly signed before attempting submission
        if signed.transaction.signatures.is_empty() {
            return Err(SignerError::InvalidTransactionState(
                "Cannot submit unsigned transaction".to_string(),
            ));
        }

        let bytes = signed.to_bytes()?;
        let sig = rpc.send_transaction(&bytes).await.map_err(|e| {
            SignerError::SigningFailed(format!("Solana RPC submission failed: {}", e))
        })?;

        info!(
            signature = %sig,
            signer = %signed.signer_pubkey,
            "Signed transaction broadcast to Solana cluster"
        );

        Ok(sig)
    }
}

// ============================================================================
// STAGE 4: PRODUCTION SIGNING LIFECYCLE COORDINATOR
// ============================================================================

/// Production-safe transaction signing lifecycle coordinator.
///
/// Enforces the complete deterministic workflow:
/// 1. Verifies authentic external signer is available and allowed in production.
/// 2. Verifies signer public key matches the fee payer and expected execution authority.
/// 3. Fetches live Solana blockhash and last_valid_block_height.
/// 4. Builds the actual transaction (guaranteeing no placeholder instructions).
/// 5. Executes preflight simulation via Solana RPC if configured/enabled by policy.
/// 6. Signs only after all deterministic checks pass.
/// 7. Records recent_blockhash and last_valid_block_height on the resulting SignedTransaction.
pub struct SigningLifecycle;

impl SigningLifecycle {
    /// Fetches a current Solana blockhash and records recent_blockhash and last_valid_block_height.
    #[instrument(skip(rpc))]
    pub async fn fetch_blockhash(rpc: &SolanaRpcClient) -> Result<(Hash, u64), SignerError> {
        let (recent_blockhash, last_valid_block_height) =
            rpc.get_latest_blockhash().await.map_err(|e| {
                SignerError::SigningFailed(format!("Failed to fetch Solana blockhash: {}", e))
            })?;

        info!(
            blockhash = %recent_blockhash,
            last_valid_block_height = last_valid_block_height,
            "Fetched current Solana blockhash and last valid block height"
        );

        Ok((recent_blockhash, last_valid_block_height))
    }

    /// Coordinates the complete production transaction signing lifecycle.
    #[instrument(skip(signer, rpc, instructions))]
    pub async fn execute_lifecycle(
        signer: &dyn ExternalSigner,
        instructions: &[Instruction],
        payer: &Pubkey,
        expected_authority: Option<&Pubkey>,
        rpc: &SolanaRpcClient,
        preflight_enabled: bool,
    ) -> Result<SignedTransaction, SignerError> {
        // 1. Signer availability check
        if !signer.is_available() {
            return Err(SignerError::SignerUnavailable {
                signer_type: signer.signer_type(),
                reason: "Signer is offline; failing closed".to_string(),
            });
        }

        // 2. Authority checks
        if signer.pubkey() != *payer {
            return Err(SignerError::AuthorityMismatch {
                expected: payer.to_string(),
                actual: signer.pubkey_string(),
            });
        }
        if let Some(expected) = expected_authority {
            if signer.pubkey() != *expected {
                return Err(SignerError::AuthorityMismatch {
                    expected: expected.to_string(),
                    actual: signer.pubkey_string(),
                });
            }
        }

        // 3. Fetch current Solana blockhash and record blockhash + last valid block height
        let (recent_blockhash, last_valid_block_height) = Self::fetch_blockhash(rpc).await?;

        // 4. Build the actual transaction (checks for empty & placeholder instructions)
        let unsigned = TransactionBuilder::build_unsigned(
            instructions,
            payer,
            recent_blockhash,
            last_valid_block_height,
        )?;

        // 5. Simulate/preflight according to production policy before signing where applicable
        if preflight_enabled {
            info!("Running preflight simulation before signing...");
            let sim_bytes = bincode::serialize(&unsigned.transaction).map_err(|e| {
                SignerError::SigningFailed(format!(
                    "Failed to serialize transaction for simulation: {}",
                    e
                ))
            })?;
            rpc.simulate_transaction(&sim_bytes)
                .await
                .map_err(|e| SignerError::PreflightSimulationFailed(format!("{}", e)))?;
            info!("Preflight simulation passed successfully; proceeding to sign");
        }

        // 6. Sign only after all deterministic checks pass
        TransactionSignerService::sign(signer, &unsigned).await
    }
}
