//! Explicit custody and beneficial equity ownership verification model.
//!
//! # Critical Shariah Distinction: Token Mint Existence != Ownership Verification
//! In conventional crypto, having a Solana SPL token mint or liquidity pool address is
//! often assumed to represent an asset. In Islamic finance (*Milkiyyah* / *Musha'*), a token
//! cannot merely track an equity price synthetically or exist as an unbacked smart contract.
//!
//! **A valid SPL token mint existing on-chain does NOT constitute ownership verification.**
//! Ownership requires affirmative, verifiable proof that:
//! 1. Regulated institutional custodians hold genuine statutory shares in insolvency-remote SPVs.
//! 2. Legal prospectuses and custody agreements legally bind the token to direct fractional ownership.
//! 3. Cryptographic evidence hashes verify that the underlying custody certificates are untampered.
//!
//! Ownership can NEVER be inferred from ticker strings, mint addresses, provider labels, or Pyth feeds.

use serde::{Deserialize, Serialize};

use crate::validation::ValidationError;

/// Audited legal and custodial ownership documentation record backing a tokenized asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnershipRecord {
    /// True if legal counsel and Shariah auditors have verified genuine asset backing.
    pub verified: bool,
    /// Legal corporate entity issuing or tokenizing the underlying instrument (e.g., "Backed Finance AG").
    pub issuer: String,
    /// Regulated qualified custodian holding the statutory shares (e.g., "Maerki Baumann & Co. AG").
    pub custodian: String,
    /// Bankruptcy-remote legal structure (e.g., "Swiss DLT Act Statutory SPV", "Liechtenstein Trust").
    pub legal_structure: String,
    /// Official statutory reference of the underlying real-world security (e.g., "ISIN: US67066G1040").
    pub instrument_reference: String,
    /// Cryptographic digest (e.g. SHA-256) of the custodial certificate and depository filing.
    pub evidence_hash: String,
    /// Unix timestamp (seconds) when custodial ownership was formally confirmed.
    pub verified_at: i64,
    /// Unix timestamp (seconds) when the custody audit determination expires.
    pub expires_at: i64,
}

impl OwnershipRecord {
    /// Validates structural integrity and completeness of the ownership record.
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_ownership(self)
    }

    /// Validates both structural integrity and temporal currency at timestamp `now`.
    pub fn validate_at(&self, now: i64) -> Result<(), ValidationError> {
        validate_ownership_at(self, now)
    }

    /// Returns true if the ownership record is verified and the evaluation timestamp `now`
    /// falls within the valid audit window (`verified_at <= now < expires_at`).
    pub fn is_current(&self, now: i64) -> bool {
        self.verified && now >= self.verified_at && now < self.expires_at
    }
}

/// Standalone function evaluating whether an [`OwnershipRecord`] is current at timestamp `now`.
pub fn is_current(record: &OwnershipRecord, now: i64) -> bool {
    record.is_current(now)
}

/// Validates that an [`OwnershipRecord`] meets all mandatory structural requirements.
///
/// # Requirements
/// - `verified == true`
/// - `issuer` must not be empty or whitespace
/// - `custodian` must not be empty or whitespace
/// - `legal_structure` must not be empty or whitespace
/// - `instrument_reference` must not be empty or whitespace
/// - `evidence_hash` must not be empty or whitespace
/// - `verified_at < expires_at`
pub fn validate_ownership(record: &OwnershipRecord) -> Result<(), ValidationError> {
    if !record.verified {
        return Err(ValidationError::InvalidParam(
            "ownership record is not verified".to_string(),
        ));
    }

    if record.issuer.trim().is_empty() {
        return Err(ValidationError::InvalidParam(
            "issuer cannot be empty".to_string(),
        ));
    }

    if record.custodian.trim().is_empty() {
        return Err(ValidationError::InvalidParam(
            "custodian cannot be empty".to_string(),
        ));
    }

    if record.legal_structure.trim().is_empty() {
        return Err(ValidationError::InvalidParam(
            "legal_structure cannot be empty".to_string(),
        ));
    }

    if record.instrument_reference.trim().is_empty() {
        return Err(ValidationError::InvalidParam(
            "instrument_reference cannot be empty".to_string(),
        ));
    }

    if record.evidence_hash.trim().is_empty() {
        return Err(ValidationError::InvalidParam(
            "evidence_hash cannot be empty".to_string(),
        ));
    }

    if record.verified_at >= record.expires_at {
        return Err(ValidationError::InvalidParam(
            "verified_at must precede expires_at".to_string(),
        ));
    }

    Ok(())
}

/// Validates structural integrity and temporal currency at timestamp `now`.
pub fn validate_ownership_at(record: &OwnershipRecord, now: i64) -> Result<(), ValidationError> {
    validate_ownership(record)?;

    if now < record.verified_at {
        return Err(ValidationError::InvalidParam(
            "current evaluation timestamp precedes verification timestamp".to_string(),
        ));
    }

    if now >= record.expires_at {
        return Err(ValidationError::InvalidParam(
            "ownership verification audit has expired".to_string(),
        ));
    }

    Ok(())
}
