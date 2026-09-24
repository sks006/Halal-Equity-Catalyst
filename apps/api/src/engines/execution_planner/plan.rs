//! Canonical execution plan produced from an approved allocation decision.
//!
//! # Core Security Principles
//! The execution plan represents an immutable, serializable, and tamper-evident specification
//! of an authorized transaction.
//!
//! The planner MUST NOT:
//! - Sign transactions
//! - Access private keys
//! - Submit transactions
//! - Bypass risk controls
//! - Modify approved quantities

use serde::{Deserialize, Serialize};
use solana_sdk::hash::hash;
use uuid::Uuid;

/// Reference oracle pricing and freshness parameters utilized during policy validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleReferenceInfo {
    /// Pyth oracle reference price scaled in micro-USD ($1.00 = 1,000,000 micro-USD)
    pub price_scaled: u64,
    /// Unix publication timestamp of the oracle price
    pub publish_time: i64,
    /// Confidence interval width in basis points
    pub conf_bps: u16,
    /// Pyth Hermes feed identifier
    pub feed_id: String,
}

/// An immutable, deterministic execution plan ready for downstream on-chain dispatch.
///
/// Contains all 13 canonical requirements:
/// 1. Asset identity (`asset_id`, `symbol`)
/// 2. Input mint (`input_mint`)
/// 3. Output mint (`output_mint`)
/// 4. Input amount (`input_amount`)
/// 5. Expected output (`expected_output_amount`)
/// 6. Minimum output (`minimum_output_amount`)
/// 7. Slippage limit (`slippage_bps`)
/// 8. Quote ID (`quote_id`)
/// 9. Quote timestamp (`quote_timestamp`)
/// 10. Oracle reference (`oracle_reference`)
/// 11. Policy decision ID (`policy_decision_id`)
/// 12. Expiry (`expires_at`)
/// 13. Nonce / Idempotency identifier (`idempotency_key`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Unique plan identifier
    pub plan_id: Uuid,
    /// Approved policy decision identifier (from ExecutionAuthorization)
    pub policy_decision_id: Uuid,
    /// Nonce / unique idempotency identifier ensuring replay protection
    pub idempotency_key: String,
    /// Target vault account address
    pub vault_address: String,
    /// Canonical asset identity (e.g. "backed:AAPLx")
    pub asset_id: String,
    /// Ticker symbol (e.g. "AAPL")
    pub symbol: String,
    /// Trade direction: true for BUY (quote -> asset), false for SELL (asset -> quote)
    pub is_buy: bool,
    /// Solana SPL token input mint
    pub input_mint: String,
    /// Solana SPL token output mint
    pub output_mint: String,
    /// Input token amount in integer atomic units
    pub input_amount: u64,
    /// Expected output token amount in integer atomic units
    pub expected_output_amount: u64,
    /// Guaranteed minimum output amount given configured slippage limit
    pub minimum_output_amount: u64,
    /// Slippage tolerance limit in basis points (e.g. 50 = 0.50%)
    pub slippage_bps: u16,
    /// DEX quote identifier
    pub quote_id: String,
    /// Unix publication timestamp of the DEX quote
    pub quote_timestamp: i64,
    /// Pyth oracle reference parameters used during policy validation
    pub oracle_reference: OracleReferenceInfo,
    /// Unix timestamp when this execution plan expires (bound by quote TTL)
    pub expires_at: i64,
    /// Unix timestamp when this plan was constructed
    pub created_at: i64,
}

impl ExecutionPlan {
    /// Domain separator for canonical execution plan byte encoding.
    pub const DOMAIN_SEPARATOR: &'static [u8] = b"EQUITY_CATALYST_EXECUTION_PLAN_V1";

    /// Produces a deterministic, canonical byte representation of the plan.
    ///
    /// Two identical plans MUST produce identical canonical bytes.
    /// Byte representation is independent of JSON whitespace, field ordering, or platform endianness.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(384);
        bytes.extend_from_slice(Self::DOMAIN_SEPARATOR);
        bytes.extend_from_slice(self.plan_id.as_bytes());
        bytes.extend_from_slice(self.policy_decision_id.as_bytes());

        // Helper for deterministic length-prefixed string encoding
        fn push_str(buf: &mut Vec<u8>, s: &str) {
            let len = s.len() as u32;
            buf.extend_from_slice(&len.to_le_bytes());
            buf.extend_from_slice(s.as_bytes());
        }

        push_str(&mut bytes, &self.idempotency_key);
        push_str(&mut bytes, &self.vault_address);
        push_str(&mut bytes, &self.asset_id);
        push_str(&mut bytes, &self.symbol);

        bytes.push(if self.is_buy { 1 } else { 0 });

        push_str(&mut bytes, &self.input_mint);
        push_str(&mut bytes, &self.output_mint);

        bytes.extend_from_slice(&self.input_amount.to_le_bytes());
        bytes.extend_from_slice(&self.expected_output_amount.to_le_bytes());
        bytes.extend_from_slice(&self.minimum_output_amount.to_le_bytes());
        bytes.extend_from_slice(&self.slippage_bps.to_le_bytes());

        push_str(&mut bytes, &self.quote_id);
        bytes.extend_from_slice(&self.quote_timestamp.to_le_bytes());

        // Oracle Reference
        bytes.extend_from_slice(&self.oracle_reference.price_scaled.to_le_bytes());
        bytes.extend_from_slice(&self.oracle_reference.publish_time.to_le_bytes());
        bytes.extend_from_slice(&self.oracle_reference.conf_bps.to_le_bytes());
        push_str(&mut bytes, &self.oracle_reference.feed_id);

        bytes.extend_from_slice(&self.expires_at.to_le_bytes());
        bytes.extend_from_slice(&self.created_at.to_le_bytes());

        bytes
    }

    /// Computes the 32-byte SHA-256 cryptographic digest of the canonical byte representation.
    pub fn canonical_digest(&self) -> [u8; 32] {
        hash(&self.to_canonical_bytes()).to_bytes()
    }

    /// Computes the hex-encoded string of the canonical digest.
    pub fn canonical_digest_hex(&self) -> String {
        let digest = self.canonical_digest();
        let mut s = String::with_capacity(64);
        for byte in digest {
            use std::fmt::Write;
            let _ = write!(&mut s, "{:02x}", byte);
        }
        s
    }

    /// Evaluates whether the plan has expired relative to timestamp `current_time`.
    pub fn is_expired(&self, current_time: i64) -> bool {
        current_time >= self.expires_at
    }

    /// Evaluates whether the plan is valid and executable at timestamp `current_time`.
    pub fn is_valid_at(&self, current_time: i64) -> bool {
        current_time >= self.created_at && current_time < self.expires_at
    }

    // Explicit convenience accessors matching exact specification names
    pub fn asset_identity(&self) -> &str {
        &self.asset_id
    }

    pub fn input_mint(&self) -> &str {
        &self.input_mint
    }

    pub fn output_mint(&self) -> &str {
        &self.output_mint
    }

    pub fn input_amount(&self) -> u64 {
        self.input_amount
    }

    pub fn expected_output(&self) -> u64 {
        self.expected_output_amount
    }

    pub fn minimum_output(&self) -> u64 {
        self.minimum_output_amount
    }

    pub fn slippage_limit(&self) -> u16 {
        self.slippage_bps
    }

    pub fn quote_id(&self) -> &str {
        &self.quote_id
    }

    pub fn quote_timestamp(&self) -> i64 {
        self.quote_timestamp
    }

    pub fn oracle_reference(&self) -> &OracleReferenceInfo {
        &self.oracle_reference
    }

    pub fn policy_decision_id(&self) -> Uuid {
        self.policy_decision_id
    }

    pub fn expiry(&self) -> i64 {
        self.expires_at
    }

    pub fn nonce(&self) -> &str {
        &self.idempotency_key
    }
}
