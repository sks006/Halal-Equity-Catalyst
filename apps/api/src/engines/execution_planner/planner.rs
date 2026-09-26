//! Deterministic Execution Planner transforming approved policy decisions into execution plans.
//!
//! # Core Security Principles
//! The planner is a pure transformation layer.
//! The planner MUST NOT:
//! - Sign transactions
//! - Access private keys
//! - Submit transactions
//! - Bypass risk controls
//! - Modify approved quantities

use equity_catalyst_jupiter::DexQuote;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use super::idempotency::{IdempotencyRecord, IdempotencyStatus, IdempotencyTracker};
use super::plan::{ExecutionPlan, OracleReferenceInfo};
use crate::engines::policy_engine::authorization::ExecutionAuthorization;

/// Structured error conditions encountered during execution planning.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PlannerError {
    #[error("DEX quote {quote_id} expired at {expires_at} (current time: {current_time})")]
    QuoteExpired {
        quote_id: String,
        expires_at: i64,
        current_time: i64,
    },

    #[error("Authorization {decision_id} expired at {valid_until} (current time: {current_time})")]
    AuthorizationExpired {
        decision_id: Uuid,
        valid_until: i64,
        current_time: i64,
    },

    #[error("Approved quantity mismatch: expected {expected}, got {got} ({reason})")]
    QuantityMismatch {
        expected: u64,
        got: u64,
        reason: String,
    },

    #[error("Token mint mismatch: expected {expected}, got {got}")]
    MintMismatch { expected: String, got: String },

    #[error("Slippage tolerance exceeded: quote price impact is {quote_bps} bps, maximum authorized is {max_allowed_bps} bps")]
    SlippageToleranceExceeded {
        quote_bps: u16,
        max_allowed_bps: u16,
    },

    #[error("Idempotency conflict for key '{key}': existing plan digest '{existing_digest}' differs from new '{new_digest}'")]
    IdempotencyConflict {
        key: String,
        existing_digest: String,
        new_digest: String,
    },

    #[error(
        "Decision {decision_id} already has an active execution plan under key '{existing_key}'"
    )]
    DuplicateExecutionPlan {
        decision_id: Uuid,
        existing_key: String,
    },

    #[error("Invalid execution plan parameter: {0}")]
    InvalidPlanParameter(String),
}

/// Deterministic planner converting an approved allocation decision into a canonical ExecutionPlan.
#[derive(Debug, Clone)]
pub struct ExecutionPlanner {
    tracker: Arc<IdempotencyTracker>,
}

impl Default for ExecutionPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionPlanner {
    /// Creates a new ExecutionPlanner with an isolated in-memory idempotency tracker.
    pub fn new() -> Self {
        Self {
            tracker: Arc::new(IdempotencyTracker::new()),
        }
    }

    /// Creates an ExecutionPlanner with a shared idempotency tracker.
    pub fn with_tracker(tracker: Arc<IdempotencyTracker>) -> Self {
        Self { tracker }
    }

    /// Accessor for the underlying idempotency tracker.
    pub fn tracker(&self) -> &Arc<IdempotencyTracker> {
        &self.tracker
    }

    /// Transforms an approved policy decision into an immutable, canonical execution plan.
    ///
    /// # Strict Security Guarantees:
    /// 1. Never signs transactions.
    /// 2. Never accesses private keys.
    /// 3. Never submits transactions to any RPC or mempool.
    /// 4. Never bypasses risk controls (enforces authorization validity, quote expiry, slippage).
    /// 5. Never modifies approved quantities (exact equality checked for sell input, guaranteed minimum for output).
    /// 6. Enforces strict idempotency and replay protection.
    pub async fn plan_execution(
        &self,
        authorization: &ExecutionAuthorization,
        quote: &DexQuote,
        pyth_feed_id: &str,
        oracle_conf_bps: u16,
        client_nonce: Option<&str>,
        current_time: i64,
    ) -> Result<ExecutionPlan, PlannerError> {
        // 1. Authorization Expiration Check
        let auth_valid_until = authorization.valid_until.timestamp();
        if current_time >= auth_valid_until {
            return Err(PlannerError::AuthorizationExpired {
                decision_id: authorization.authorization_id,
                valid_until: auth_valid_until,
                current_time,
            });
        }

        // 2. DEX Quote Expiration Check
        if quote.is_expired(current_time) {
            return Err(PlannerError::QuoteExpired {
                quote_id: quote.quote_id.clone(),
                expires_at: quote.expires_at,
                current_time,
            });
        }

        // 3. Mint Consistency Checks
        if authorization.is_buy {
            if quote.output_mint != authorization.mint_address {
                return Err(PlannerError::MintMismatch {
                    expected: authorization.mint_address.clone(),
                    got: quote.output_mint.clone(),
                });
            }
        } else {
            if quote.input_mint != authorization.mint_address {
                return Err(PlannerError::MintMismatch {
                    expected: authorization.mint_address.clone(),
                    got: quote.input_mint.clone(),
                });
            }
        }

        // 4. Strict Quantity Preservation Checks
        // INVARIANT: The planner must NOT modify the approved quantities.
        if !authorization.is_buy {
            // SELL Trade: input token amount must EXACTLY match the authorized amount
            if quote.input_amount != authorization.authorized_amount {
                return Err(PlannerError::QuantityMismatch {
                    expected: authorization.authorized_amount,
                    got: quote.input_amount,
                    reason: "Sell trade input amount must exactly equal authorized asset amount"
                        .to_string(),
                });
            }
            // Guaranteed minimum output must honor or exceed the authorized minimum output
            if quote.minimum_output_amount < authorization.dex_minimum_output_amount {
                return Err(PlannerError::QuantityMismatch {
                    expected: authorization.dex_minimum_output_amount,
                    got: quote.minimum_output_amount,
                    reason: "Quote minimum output is lower than authorized minimum output"
                        .to_string(),
                });
            }
        } else {
            // BUY Trade: guaranteed minimum output must honor or exceed authorized minimum output
            if quote.minimum_output_amount < authorization.dex_minimum_output_amount {
                return Err(PlannerError::QuantityMismatch {
                    expected: authorization.dex_minimum_output_amount,
                    got: quote.minimum_output_amount,
                    reason: "Quote minimum output is lower than authorized minimum output"
                        .to_string(),
                });
            }
        }

        // 5. Slippage & Price Impact Check
        if quote.price_impact_bps > authorization.max_slippage_bps {
            return Err(PlannerError::SlippageToleranceExceeded {
                quote_bps: quote.price_impact_bps,
                max_allowed_bps: authorization.max_slippage_bps,
            });
        }

        // 6. Expiry Determination (bound by the earlier of quote and authorization expiry)
        let plan_expiry = quote.expires_at.min(auth_valid_until);

        // 7. Nonce / Idempotency Key Determination
        let idempotency_key = match client_nonce {
            Some(n) if !n.trim().is_empty() => {
                IdempotencyTracker::generate_key(&authorization.authorization_id, n)
            }
            _ => format!(
                "idemp:{}:{}",
                authorization.authorization_id, quote.quote_id
            ),
        };

        // Deterministic plan_id derived from decision ID and idempotency key
        let plan_id = {
            let mut seed = Vec::with_capacity(64);
            seed.extend_from_slice(authorization.authorization_id.as_bytes());
            seed.extend_from_slice(idempotency_key.as_bytes());
            let hash_bytes = solana_sdk::hash::hash(&seed).to_bytes();
            let mut uuid_bytes = [0u8; 16];
            uuid_bytes.copy_from_slice(&hash_bytes[0..16]);
            Uuid::from_bytes(uuid_bytes)
        };

        // 8. Construct Candidate ExecutionPlan
        let plan = ExecutionPlan {
            plan_id,
            policy_decision_id: authorization.authorization_id,
            idempotency_key: idempotency_key.clone(),
            vault_address: authorization.vault_address.clone(),
            asset_id: authorization.asset_id.clone(),
            symbol: authorization.symbol.clone(),
            is_buy: authorization.is_buy,
            input_mint: quote.input_mint.clone(),
            output_mint: quote.output_mint.clone(),
            input_amount: quote.input_amount,
            expected_output_amount: quote.expected_output_amount,
            minimum_output_amount: quote.minimum_output_amount,
            slippage_bps: authorization.max_slippage_bps,
            quote_id: quote.quote_id.clone(),
            quote_timestamp: quote.quote_timestamp,
            oracle_reference: OracleReferenceInfo {
                price_scaled: authorization.oracle_reference_price_scaled,
                publish_time: authorization.oracle_publish_time,
                conf_bps: oracle_conf_bps,
                feed_id: pyth_feed_id.to_string(),
            },
            expires_at: plan_expiry,
            created_at: current_time,
        };

        // 9. Register in Idempotency Tracker
        let record = IdempotencyRecord {
            idempotency_key,
            policy_decision_id: authorization.authorization_id,
            plan_digest: plan.canonical_digest_hex(),
            status: IdempotencyStatus::Planned,
            created_at: current_time,
            expires_at: plan_expiry,
        };

        self.tracker
            .check_and_register(record, current_time)
            .await?;

        Ok(plan)
    }
}
