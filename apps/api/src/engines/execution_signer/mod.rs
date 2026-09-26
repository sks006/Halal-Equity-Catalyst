//! Isolated cryptographic execution signer and transaction lifecycle module.
//!
//! # Core Security Principles
//! - Real cryptographic Ed25519 signatures (never `DefaultHasher` or fake hashes).
//! - No hardcoded fallback seeds (`[42u8; 32]`) in production code paths.
//! - Fails closed when the signer is offline or unavailable.
//! - Strict separation of:
//!   1. Transaction Construction
//!   2. Transaction Signing
//!   3. Transaction Submission
//! - Execution planner never receives raw private keys.

pub mod error;
pub mod external;
pub mod pipeline;

pub use error::SignerError;
pub use external::{
    load_production_signer, DevTestSigner, ExternalSigner, HsmSigner, KeypairSigner, KmsSigner,
    RemoteHsmSigner, UnavailableSigner,
};
pub use pipeline::{
    is_placeholder_instruction, SignedTransaction, SigningLifecycle, TransactionBuilder,
    TransactionSignerService, TransactionSubmitter, UnsignedTransaction, PLACEHOLDER_PROGRAM_ID,
};
