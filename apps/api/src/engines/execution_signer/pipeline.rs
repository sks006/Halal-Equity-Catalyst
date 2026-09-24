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
    hash::Hash,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use tracing::{info, instrument};

use super::error::SignerError;
use super::external::ExternalSigner;
use crate::engines::execution_planner::ExecutionPlan;

// ============================================================================
// STAGE 1: TRANSACTION CONSTRUCTION
// ============================================================================

/// An un-signed Solana transaction with explicit message and instructions.
#[derive(Debug, Clone, PartialEq)]
pub struct UnsignedTransaction {
    pub transaction: Transaction,
    pub payer: Pubkey,
    pub instructions: Vec<Instruction>,
    pub recent_blockhash: Hash,
}

/// Pure transaction construction engine.
///
/// Builds instruction messages and transaction framing without signing or submitting.
pub struct TransactionBuilder;

impl TransactionBuilder {
    /// Constructs an unsigned transaction from raw instructions, fee payer, and recent blockhash.
    pub fn build_unsigned(
        instructions: &[Instruction],
        payer: &Pubkey,
        recent_blockhash: Hash,
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

        let message = Message::new(instructions, Some(payer));
        let transaction = Transaction::new_unsigned(message);

        Ok(UnsignedTransaction {
            transaction,
            payer: *payer,
            instructions: instructions.to_vec(),
            recent_blockhash,
        })
    }

    /// Constructs an unsigned transaction representing an approved ExecutionPlan trade.
    pub fn build_from_plan(
        plan: &ExecutionPlan,
        payer: &Pubkey,
        recent_blockhash: Hash,
        custom_instructions: &[Instruction],
    ) -> Result<UnsignedTransaction, SignerError> {
        if custom_instructions.is_empty() {
            // Note: Downstream Anchor/DEX quoter would supply the DEX swap instruction.
            // When mock or plan testing, build a canonical placeholder instruction.
            let memo_ix = Instruction::new_with_bytes(
                solana_sdk::pubkey::Pubkey::new_unique(),
                &plan.to_canonical_bytes(),
                vec![],
            );
            Self::build_unsigned(&[memo_ix], payer, recent_blockhash)
        } else {
            Self::build_unsigned(custom_instructions, payer, recent_blockhash)
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
            "Transaction cryptographically signed"
        );

        Ok(SignedTransaction {
            transaction: tx,
            primary_signature,
            signer_pubkey: signer.pubkey(),
            signer_type: signer.signer_type(),
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
