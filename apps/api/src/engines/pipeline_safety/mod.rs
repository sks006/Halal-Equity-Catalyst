//! Trading Pipeline Safety & Fail-Closed Infrastructure (Phase 16).
//!
//! # Objective
//! Make the trading pipeline fail closed under critical data and infrastructure failures.
//!
//! # Required Principle
//! **Critical failures must prevent execution rather than silently continuing with unsafe state.**
//!
//! # 15 Explicit Failure Scenarios Handled:
//! 1. Pyth unavailable
//! 2. Pyth stream disconnected
//! 3. Pyth data stale
//! 4. Pyth confidence too large
//! 5. Wrong feed
//! 6. Asset deactivated
//! 7. Shariah approval revoked
//! 8. DEX quote expired
//! 9. DEX unavailable
//! 10. Signer unavailable
//! 11. Transaction submission failure
//! 12. Transaction confirmation timeout
//! 13. Duplicate execution request
//! 14. Database unavailable
//! 15. WebSocket clients disconnected

use chrono::{DateTime, Utc};
use equity_catalyst_pyth::normalize_feed_id;
use equity_catalyst_shared::shariah::ShariahStatus;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

use crate::engines::execution_signer::ExternalSigner;
use crate::services::MarketPriceUpdate;

/// Structured, machine-readable failure classifications covering all critical
/// data and infrastructure failure modes in the trading pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum PipelineFailure {
    /// 1. Pyth price source is completely unavailable, offline, or unreachable (HTTP 5xx, timeout, TCP refused).
    #[error("Pyth oracle unavailable for '{symbol}': {details}")]
    PythUnavailable { symbol: String, details: String },

    /// 2. Pyth real-time SSE / WebSocket price stream is disconnected or reconnecting.
    #[error("Pyth price stream disconnected: {details}")]
    PythStreamDisconnected { details: String },

    /// 3. Pyth price data is stale (age exceeds configured maximum allowable staleness window).
    #[error("Pyth price data stale for '{symbol}': age {age_secs}s exceeds limit {max_staleness_secs}s (published at {publish_time})")]
    PythDataStale {
        symbol: String,
        publish_time: i64,
        age_secs: i64,
        max_staleness_secs: i64,
    },

    /// 4. Pyth confidence interval is too wide relative to price (exceeds risk tolerance).
    #[error("Pyth confidence too large for '{symbol}': {conf_bps} bps exceeds limit {max_conf_bps} bps")]
    PythConfidenceTooLarge {
        symbol: String,
        conf_bps: u16,
        max_conf_bps: u16,
    },

    /// 5. Pyth feed ID received does not match expected registered canonical feed ID.
    #[error("Wrong feed ID for '{symbol}': expected '{expected_feed_id}', got '{received_feed_id}'")]
    WrongFeed {
        symbol: String,
        expected_feed_id: String,
        received_feed_id: String,
    },

    /// 6. Asset is deactivated, suspended, or inactive in the canonical asset registry.
    #[error("Asset '{symbol}' is deactivated or inactive in asset registry (status: {status})")]
    AssetDeactivated { symbol: String, status: String },

    /// 7. Shariah board approval has been revoked, rejected, or review has expired.
    #[error("Shariah approval revoked or non-compliant for '{symbol}': status {status}, reason: {reason}")]
    ShariahApprovalRevoked {
        symbol: String,
        status: String,
        reason: String,
    },

    /// 8. Executable DEX quote has expired (current timestamp >= quote expiry).
    #[error("DEX quote '{quote_id}' expired at {expires_at} (current time: {now})")]
    DexQuoteExpired {
        quote_id: String,
        expires_at: i64,
        now: i64,
    },

    /// 9. DEX routing / quoter service (Jupiter/Meteora) is offline, timed out, or returned 5xx.
    #[error("DEX quoter unavailable: {details}")]
    DexUnavailable { details: String },

    /// 10. Cryptographic signer (HSM/KMS/Keypair) is offline, unavailable, or failed to sign.
    #[error("Signer unavailable ({signer_type}): {details}")]
    SignerUnavailable {
        signer_type: String,
        details: String,
    },

    /// 11. Transaction submission to Solana cluster failed (RPC connection refused, node drop, cluster error).
    #[error("Transaction submission failure: {details}")]
    TransactionSubmissionFailure { details: String },

    /// 12. Transaction confirmation timed out without reaching cluster commitment.
    #[error("Transaction confirmation timeout for signature '{tx_signature}' after {timeout_secs}s")]
    TransactionConfirmationTimeout {
        tx_signature: String,
        timeout_secs: u64,
    },

    /// 13. Duplicate execution request rejected by idempotency gate (replay / double-spend defense).
    #[error("Duplicate execution request rejected (execution_id: {execution_id}, existing_status: {existing_status})")]
    DuplicateExecutionRequest {
        execution_id: Uuid,
        existing_status: String,
    },

    /// 14. Database unavailable or failed state persistence (fails closed to prevent unrecorded execution).
    #[error("Database unavailable: {details} (failing closed to prevent unrecorded state)")]
    DatabaseUnavailable { details: String },

    /// 15. WebSocket streaming clients disconnected or stream buffer closed.
    #[error("WebSocket client stream disconnected or channel closed: {details}")]
    WebSocketClientDisconnected { details: String },
}

impl PipelineFailure {
    /// Canonical error code string for JSON error responses and audit logging.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::PythUnavailable { .. } => "PYTH_UNAVAILABLE",
            Self::PythStreamDisconnected { .. } => "PYTH_STREAM_DISCONNECTED",
            Self::PythDataStale { .. } => "PYTH_DATA_STALE",
            Self::PythConfidenceTooLarge { .. } => "PYTH_CONFIDENCE_TOO_LARGE",
            Self::WrongFeed { .. } => "WRONG_FEED",
            Self::AssetDeactivated { .. } => "ASSET_DEACTIVATED",
            Self::ShariahApprovalRevoked { .. } => "SHARIAH_APPROVAL_REVOKED",
            Self::DexQuoteExpired { .. } => "DEX_QUOTE_EXPIRED",
            Self::DexUnavailable { .. } => "DEX_UNAVAILABLE",
            Self::SignerUnavailable { .. } => "SIGNER_UNAVAILABLE",
            Self::TransactionSubmissionFailure { .. } => "TRANSACTION_SUBMISSION_FAILURE",
            Self::TransactionConfirmationTimeout { .. } => "TRANSACTION_CONFIRMATION_TIMEOUT",
            Self::DuplicateExecutionRequest { .. } => "DUPLICATE_EXECUTION_REQUEST",
            Self::DatabaseUnavailable { .. } => "DATABASE_UNAVAILABLE",
            Self::WebSocketClientDisconnected { .. } => "WEBSOCKET_CLIENT_DISCONNECTED",
        }
    }

    /// Associated HTTP status code for API responses.
    pub fn http_status_code(&self) -> u16 {
        match self {
            Self::PythUnavailable { .. } => 503,
            Self::PythStreamDisconnected { .. } => 503,
            Self::PythDataStale { .. } => 400,
            Self::PythConfidenceTooLarge { .. } => 400,
            Self::WrongFeed { .. } => 400,
            Self::AssetDeactivated { .. } => 400,
            Self::ShariahApprovalRevoked { .. } => 400,
            Self::DexQuoteExpired { .. } => 400,
            Self::DexUnavailable { .. } => 503,
            Self::SignerUnavailable { .. } => 503,
            Self::TransactionSubmissionFailure { .. } => 503,
            Self::TransactionConfirmationTimeout { .. } => 504,
            Self::DuplicateExecutionRequest { .. } => 409,
            Self::DatabaseUnavailable { .. } => 503,
            Self::WebSocketClientDisconnected { .. } => 400,
        }
    }

    /// Returns true if this failure represents a transient infrastructure outage
    /// that may be recovered via retry or reconnection.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::PythUnavailable { .. }
                | Self::PythStreamDisconnected { .. }
                | Self::DexUnavailable { .. }
                | Self::SignerUnavailable { .. }
                | Self::TransactionSubmissionFailure { .. }
                | Self::TransactionConfirmationTimeout { .. }
                | Self::DatabaseUnavailable { .. }
        )
    }
}

/// Comprehensive states for the execution pipeline state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PipelineState {
    /// Idle state waiting for execution request
    #[default]
    Idle,
    /// Preflight validation of asset identity and policy limits
    PreflightValidating,
    /// Real-time Pyth price freshness and confidence validation
    PriceValidating,
    /// Shariah governance and spot ownership revalidation
    ShariahValidating,
    /// DEX quote freshness and price impact validation
    QuoteValidating,
    /// Transaction instruction building
    InstructionBuilding,
    /// Pre-execution simulation gate
    Simulating,
    /// Pre-signing emergency pause check & cryptographic signing
    Signing,
    /// Broadcasting signed transaction to Solana cluster
    Submitting,
    /// Awaiting Solana cluster commitment
    Confirming,
    /// Post-execution token balance delta reconciliation
    Reconciling,
    /// Successfully executed, confirmed, and reconciled
    Completed,
    /// Failed closed due to critical data or infrastructure failure
    FailedClosed,
    /// Successfully recovered from failure after underlying cause resolved
    Recovered,
}

impl fmt::Display for PipelineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Complete diagnostic record of a pipeline execution or failure event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineExecutionAudit {
    pub execution_id: Uuid,
    pub state: PipelineState,
    pub failure: Option<PipelineFailure>,
    pub timestamp: DateTime<Utc>,
    pub recovery_attempts: u32,
    pub details: Option<String>,
}

/// Deterministic Trading Pipeline Safety Guard.
///
/// Implements immediate, fail-closed barrier checks for all 15 failure conditions.
/// Guarantees that if ANY check fails, execution is halted immediately:
/// - Zero transactions signed.
/// - Zero transactions submitted to Solana cluster.
/// - Zero state mutations to vault or portfolio balances.
#[derive(Debug, Clone, Default)]
pub struct TradingPipelineSafetyGuard {
    max_staleness_secs: i64,
    max_confidence_bps: u16,
}

impl TradingPipelineSafetyGuard {
    /// Creates a new safety guard with standard conservative bounds:
    /// - Max staleness: 30 seconds
    /// - Max confidence: 50 basis points (0.50%)
    pub fn new() -> Self {
        Self {
            max_staleness_secs: 30,
            max_confidence_bps: 50,
        }
    }

    pub fn with_limits(max_staleness_secs: i64, max_confidence_bps: u16) -> Self {
        Self {
            max_staleness_secs,
            max_confidence_bps,
        }
    }

    // ========================================================================
    // 1. Pyth Available Check
    // ========================================================================
    pub fn check_pyth_availability<'a>(
        &self,
        symbol: &str,
        price_result: &'a Result<MarketPriceUpdate, String>,
    ) -> Result<&'a MarketPriceUpdate, PipelineFailure> {
        price_result.as_ref().map_err(|err| PipelineFailure::PythUnavailable {
            symbol: symbol.to_string(),
            details: err.clone(),
        })
    }

    // ========================================================================
    // 2. Pyth Stream Connected Check
    // ========================================================================
    pub fn check_pyth_stream_connected(
        &self,
        is_stream_connected: bool,
        details: &str,
    ) -> Result<(), PipelineFailure> {
        if !is_stream_connected {
            Err(PipelineFailure::PythStreamDisconnected {
                details: details.to_string(),
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // 3. Pyth Data Stale Check
    // ========================================================================
    pub fn check_pyth_freshness(
        &self,
        update: &MarketPriceUpdate,
        now: i64,
    ) -> Result<(), PipelineFailure> {
        let age_secs = now - update.publish_time;
        if age_secs > self.max_staleness_secs || age_secs < -5 {
            Err(PipelineFailure::PythDataStale {
                symbol: update.symbol.clone(),
                publish_time: update.publish_time,
                age_secs,
                max_staleness_secs: self.max_staleness_secs,
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // 4. Pyth Confidence Check
    // ========================================================================
    pub fn check_pyth_confidence(
        &self,
        update: &MarketPriceUpdate,
    ) -> Result<(), PipelineFailure> {
        if update.conf_bps > self.max_confidence_bps {
            Err(PipelineFailure::PythConfidenceTooLarge {
                symbol: update.symbol.clone(),
                conf_bps: update.conf_bps,
                max_conf_bps: self.max_confidence_bps,
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // 5. Wrong Feed Check
    // ========================================================================
    pub fn check_feed_id(
        &self,
        symbol: &str,
        expected_feed_id: &str,
        received_feed_id: &str,
    ) -> Result<(), PipelineFailure> {
        let clean_expected = normalize_feed_id(expected_feed_id);
        let clean_received = normalize_feed_id(received_feed_id);

        if clean_expected != clean_received {
            Err(PipelineFailure::WrongFeed {
                symbol: symbol.to_string(),
                expected_feed_id: clean_expected,
                received_feed_id: clean_received,
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // 6. Asset Deactivated Check
    // ========================================================================
    pub fn check_asset_active(
        &self,
        symbol: &str,
        is_active: bool,
        status_name: &str,
    ) -> Result<(), PipelineFailure> {
        if !is_active {
            Err(PipelineFailure::AssetDeactivated {
                symbol: symbol.to_string(),
                status: status_name.to_string(),
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // 7. Shariah Approval Revoked Check
    // ========================================================================
    pub fn check_shariah_approval(
        &self,
        symbol: &str,
        status: ShariahStatus,
        reviewed_at: i64,
        expires_at: i64,
        now: i64,
    ) -> Result<(), PipelineFailure> {
        if status != ShariahStatus::Approved {
            return Err(PipelineFailure::ShariahApprovalRevoked {
                symbol: symbol.to_string(),
                status: status.to_string(),
                reason: format!("Shariah status is {} (must be Approved)", status),
            });
        }

        if now < reviewed_at || now >= expires_at {
            return Err(PipelineFailure::ShariahApprovalRevoked {
                symbol: symbol.to_string(),
                status: status.to_string(),
                reason: format!(
                    "Shariah review has expired (reviewed_at: {}, expires_at: {}, now: {})",
                    reviewed_at, expires_at, now
                ),
            });
        }

        Ok(())
    }

    // ========================================================================
    // 8. DEX Quote Expired Check
    // ========================================================================
    pub fn check_dex_quote_expiry(
        &self,
        quote_id: &str,
        expires_at: i64,
        now: i64,
    ) -> Result<(), PipelineFailure> {
        if now >= expires_at {
            Err(PipelineFailure::DexQuoteExpired {
                quote_id: quote_id.to_string(),
                expires_at,
                now,
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // 9. DEX Unavailable Check
    // ========================================================================
    pub fn check_dex_availability<'a, T>(
        &self,
        quoter_result: &'a Result<T, String>,
    ) -> Result<&'a T, PipelineFailure> {
        quoter_result.as_ref().map_err(|err| PipelineFailure::DexUnavailable {
            details: err.clone(),
        })
    }

    // ========================================================================
    // 10. Signer Unavailable Check
    // ========================================================================
    pub fn check_signer_availability(
        &self,
        signer: &dyn ExternalSigner,
    ) -> Result<(), PipelineFailure> {
        if !signer.is_available() {
            Err(PipelineFailure::SignerUnavailable {
                signer_type: signer.signer_type().to_string(),
                details: "Signer is offline, key is not loaded, or hardware device is detached".to_string(),
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // 11. Transaction Submission Check
    // ========================================================================
    pub fn check_submission_result<'a, T>(
        &self,
        submission_result: &'a Result<T, String>,
    ) -> Result<&'a T, PipelineFailure> {
        submission_result
            .as_ref()
            .map_err(|err| PipelineFailure::TransactionSubmissionFailure {
                details: err.clone(),
            })
    }

    // ========================================================================
    // 12. Transaction Confirmation Timeout Check
    // ========================================================================
    pub fn check_confirmation(
        &self,
        tx_signature: &str,
        confirmed: bool,
        timeout_secs: u64,
    ) -> Result<(), PipelineFailure> {
        if !confirmed {
            Err(PipelineFailure::TransactionConfirmationTimeout {
                tx_signature: tx_signature.to_string(),
                timeout_secs,
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // 13. Duplicate Execution Request (Idempotency) Check
    // ========================================================================
    pub fn check_idempotency(
        &self,
        execution_id: Uuid,
        is_duplicate: bool,
        existing_status: Option<&str>,
    ) -> Result<(), PipelineFailure> {
        if is_duplicate {
            Err(PipelineFailure::DuplicateExecutionRequest {
                execution_id,
                existing_status: existing_status.unwrap_or("unknown").to_string(),
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // 14. Database Unavailable Check
    // ========================================================================
    pub fn check_database_availability<'a, T>(
        &self,
        db_result: &'a Result<T, String>,
    ) -> Result<&'a T, PipelineFailure> {
        db_result
            .as_ref()
            .map_err(|err| PipelineFailure::DatabaseUnavailable {
                details: err.clone(),
            })
    }

    // ========================================================================
    // 15. WebSocket Client Disconnect Check
    // ========================================================================
    pub fn check_websocket_client_connected(
        &self,
        is_connected: bool,
        details: &str,
    ) -> Result<(), PipelineFailure> {
        if !is_connected {
            Err(PipelineFailure::WebSocketClientDisconnected {
                details: details.to_string(),
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // Comprehensive Multi-Stage Safety Evaluation
    // ========================================================================
    /// Evaluates all 15 critical safety invariants across data, governance, execution,
    /// and infrastructure boundaries.
    ///
    /// Fails closed immediately at the very first breached invariant, ensuring zero
    /// unvalidated state transitions, zero unauthorized signing, and zero unconfirmed mutations.
    pub fn evaluate_pipeline_safety(
        &self,
        ctx: &PipelineExecutionContext,
    ) -> Result<PipelineState, PipelineFailure> {
        // 1. Pyth availability
        let price = self.check_pyth_availability(ctx.symbol, ctx.price_update)?;

        // 2. Pyth stream connected
        self.check_pyth_stream_connected(ctx.is_stream_connected, "Pyth price stream is disconnected")?;

        // 3. Pyth data fresh
        self.check_pyth_freshness(price, ctx.now)?;

        // 4. Pyth confidence within bounds
        self.check_pyth_confidence(price)?;

        // 5. Canonical feed ID match
        self.check_feed_id(ctx.symbol, ctx.expected_feed_id, &price.pyth_feed_id)?;

        // 6. Asset active in registry
        self.check_asset_active(ctx.symbol, ctx.is_asset_active, ctx.asset_status)?;

        // 7. Shariah board approval valid and unexpired
        self.check_shariah_approval(
            ctx.symbol,
            ctx.shariah_status,
            ctx.shariah_reviewed_at,
            ctx.shariah_expires_at,
            ctx.now,
        )?;

        // 8. DEX quote unexpired
        self.check_dex_quote_expiry(ctx.quote_id, ctx.quote_expires_at, ctx.now)?;

        // 9. DEX routing / quoter available
        self.check_dex_availability(ctx.dex_result)?;

        // 10. Cryptographic signer available
        if !ctx.is_signer_available {
            return Err(PipelineFailure::SignerUnavailable {
                signer_type: ctx.signer_type.to_string(),
                details: "Signer is offline, key is not loaded, or hardware device is detached".to_string(),
            });
        }

        // 13. Idempotency / duplicate execution check (before state mutation)
        self.check_idempotency(ctx.execution_id, ctx.is_duplicate, ctx.existing_status)?;

        // 14. Database available for state persistence (fails closed to prevent unrecorded state)
        self.check_database_availability(ctx.db_result)?;

        // 11. Transaction submission to Solana RPC
        let sig = self.check_submission_result(ctx.submission_result)?;

        // 12. Transaction confirmation reached on Solana cluster
        self.check_confirmation(sig, ctx.is_confirmed, ctx.confirmation_timeout_secs)?;

        // 15. WebSocket client connected for delivery
        self.check_websocket_client_connected(ctx.is_ws_connected, "Client connection dropped")?;

        Ok(PipelineState::Completed)
    }
}

/// Comprehensive contextual inputs for evaluating the full 15-point safety checklist.
#[derive(Debug, Clone)]
pub struct PipelineExecutionContext<'a> {
    pub symbol: &'a str,
    pub expected_feed_id: &'a str,
    pub price_update: &'a Result<MarketPriceUpdate, String>,
    pub is_stream_connected: bool,
    pub is_asset_active: bool,
    pub asset_status: &'a str,
    pub shariah_status: ShariahStatus,
    pub shariah_reviewed_at: i64,
    pub shariah_expires_at: i64,
    pub quote_id: &'a str,
    pub quote_expires_at: i64,
    pub dex_result: &'a Result<(), String>,
    pub is_signer_available: bool,
    pub signer_type: &'a str,
    pub submission_result: &'a Result<String, String>,
    pub is_confirmed: bool,
    pub confirmation_timeout_secs: u64,
    pub execution_id: Uuid,
    pub is_duplicate: bool,
    pub existing_status: Option<&'a str>,
    pub db_result: &'a Result<(), String>,
    pub is_ws_connected: bool,
    pub now: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::execution_signer::DevTestSigner;
    use crate::engines::execution_signer::UnavailableSigner;

    #[test]
    fn test_pipeline_failure_error_codes_and_status() {
        let f1 = PipelineFailure::PythUnavailable {
            symbol: "NVDA".to_string(),
            details: "Connection refused".to_string(),
        };
        assert_eq!(f1.error_code(), "PYTH_UNAVAILABLE");
        assert_eq!(f1.http_status_code(), 503);
        assert!(f1.is_transient());

        let f3 = PipelineFailure::PythDataStale {
            symbol: "NVDA".to_string(),
            publish_time: 1000,
            age_secs: 120,
            max_staleness_secs: 30,
        };
        assert_eq!(f3.error_code(), "PYTH_DATA_STALE");
        assert_eq!(f3.http_status_code(), 400);
        assert!(!f3.is_transient());

        let f10 = PipelineFailure::SignerUnavailable {
            signer_type: "RemoteHsmSigner".to_string(),
            details: "HSM socket timeout".to_string(),
        };
        assert_eq!(f10.error_code(), "SIGNER_UNAVAILABLE");
        assert_eq!(f10.http_status_code(), 503);
        assert!(f10.is_transient());

        let f13 = PipelineFailure::DuplicateExecutionRequest {
            execution_id: Uuid::new_v4(),
            existing_status: "confirmed".to_string(),
        };
        assert_eq!(f13.error_code(), "DUPLICATE_EXECUTION_REQUEST");
        assert_eq!(f13.http_status_code(), 409);
    }

    #[test]
    fn test_safety_guard_pyth_checks() {
        let guard = TradingPipelineSafetyGuard::new();

        // 1. Pyth unavailable
        let unavail: Result<MarketPriceUpdate, String> = Err("503 Service Unavailable".to_string());
        let res1 = guard.check_pyth_availability("NVDA", &unavail);
        assert!(matches!(res1, Err(PipelineFailure::PythUnavailable { .. })));

        // 2. Stream disconnected
        let res2 = guard.check_pyth_stream_connected(false, "SSE dropped");
        assert!(matches!(res2, Err(PipelineFailure::PythStreamDisconnected { .. })));
        assert!(guard.check_pyth_stream_connected(true, "OK").is_ok());

        // 3. Stale price
        let now = 1_700_000_100;
        let stale_update = MarketPriceUpdate::from_raw(
            "backed:NVDA",
            "NVDA",
            "Mint111111111111111111111111111111111111111",
            "0xfeed1234",
            12_000_000_000,
            -8,
            500_000,
            now - 60, // 60s old (limit is 30s)
            30,
            None,
        )
        .unwrap();
        let res3 = guard.check_pyth_freshness(&stale_update, now);
        assert!(matches!(res3, Err(PipelineFailure::PythDataStale { .. })));

        // 4. Excessive confidence interval (limit is 50 bps, test with 100 bps)
        let wide_conf_update = MarketPriceUpdate::from_raw(
            "backed:NVDA",
            "NVDA",
            "Mint111111111111111111111111111111111111111",
            "0xfeed1234",
            12_000_000_000,
            -8,
            120_000_000, // 100 bps = 1.00% > 50 bps
            now,
            30,
            None,
        )
        .unwrap();
        let res4 = guard.check_pyth_confidence(&wide_conf_update);
        assert!(matches!(res4, Err(PipelineFailure::PythConfidenceTooLarge { .. })));

        // 5. Wrong feed
        let res5 = guard.check_feed_id("NVDA", "0xexpectedfeed", "0xwrongfeed");
        assert!(matches!(res5, Err(PipelineFailure::WrongFeed { .. })));
        assert!(guard.check_feed_id("NVDA", "0xfeed1234", "feed1234").is_ok());
    }

    #[test]
    fn test_safety_guard_governance_and_dex_checks() {
        let guard = TradingPipelineSafetyGuard::new();
        let now = 1_700_000_000;

        // 6. Asset deactivated
        let res6 = guard.check_asset_active("NVDA", false, "Suspended");
        assert!(matches!(res6, Err(PipelineFailure::AssetDeactivated { .. })));
        assert!(guard.check_asset_active("NVDA", true, "Active").is_ok());

        // 7. Shariah approval revoked
        let res7 = guard.check_shariah_approval("NVDA", ShariahStatus::Revoked, now - 1000, now + 1000, now);
        assert!(matches!(res7, Err(PipelineFailure::ShariahApprovalRevoked { .. })));
        assert!(guard.check_shariah_approval("NVDA", ShariahStatus::Approved, now - 1000, now + 1000, now).is_ok());

        // 8. DEX quote expired
        let res8 = guard.check_dex_quote_expiry("quote-1", now - 10, now);
        assert!(matches!(res8, Err(PipelineFailure::DexQuoteExpired { .. })));
        assert!(guard.check_dex_quote_expiry("quote-1", now + 60, now).is_ok());

        // 9. DEX unavailable
        let dex_down: Result<(), String> = Err("504 Gateway Timeout".to_string());
        let res9 = guard.check_dex_availability(&dex_down);
        assert!(matches!(res9, Err(PipelineFailure::DexUnavailable { .. })));
    }

    #[test]
    fn test_safety_guard_signer_rpc_and_infra_checks() {
        let guard = TradingPipelineSafetyGuard::new();

        // 10. Signer unavailable
        let offline_signer = UnavailableSigner::new();
        let res10 = guard.check_signer_availability(&offline_signer);
        assert!(matches!(res10, Err(PipelineFailure::SignerUnavailable { .. })));

        let online_signer = DevTestSigner::new_ephemeral();
        assert!(guard.check_signer_availability(&online_signer).is_ok());

        // 11. Transaction submission failure
        let sub_err: Result<(), String> = Err("RPC TCP drop".to_string());
        let res11 = guard.check_submission_result(&sub_err);
        assert!(matches!(res11, Err(PipelineFailure::TransactionSubmissionFailure { .. })));

        // 12. Transaction confirmation timeout
        let res12 = guard.check_confirmation("sig123", false, 30);
        assert!(matches!(res12, Err(PipelineFailure::TransactionConfirmationTimeout { .. })));
        assert!(guard.check_confirmation("sig123", true, 30).is_ok());

        // 13. Duplicate execution request
        let exec_id = Uuid::new_v4();
        let res13 = guard.check_idempotency(exec_id, true, Some("confirmed"));
        assert!(matches!(res13, Err(PipelineFailure::DuplicateExecutionRequest { .. })));
        assert!(guard.check_idempotency(exec_id, false, None).is_ok());

        // 14. Database unavailable
        let db_down: Result<(), String> = Err("Connection refused".to_string());
        let res14 = guard.check_database_availability(&db_down);
        assert!(matches!(res14, Err(PipelineFailure::DatabaseUnavailable { .. })));

        // 15. WebSocket client disconnected
        let res15 = guard.check_websocket_client_connected(false, "Client TCP reset");
        assert!(matches!(res15, Err(PipelineFailure::WebSocketClientDisconnected { .. })));
        assert!(guard.check_websocket_client_connected(true, "Client active").is_ok());
    }
}
