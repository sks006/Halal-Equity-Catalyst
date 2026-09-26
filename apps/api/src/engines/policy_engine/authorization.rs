//! Deterministic policy and risk validation layer between allocation proposals and execution.
//!
//! # Core Security Principle
//! **AI may propose. AI may NOT authorize execution, bypass risk controls,
//! bypass Shariah rules, choose arbitrary signing keys, or directly submit transactions.**
//!
//! Every proposal must satisfy 13 deterministic criteria to be granted an `ExecutionAuthorization`.
//! Any AI-generated boolean (such as `ai_approved = true`) is strictly quarantined and ignored.

use chrono::{DateTime, Utc};
use equity_catalyst_jupiter::DexQuote;
use equity_catalyst_shared::shariah::{RegisteredAsset, ShariahAssetRegistry, ShariahStatus};
use serde::{Deserialize, Serialize};
use solana_sdk::hash::hash;
use uuid::Uuid;

use crate::{
    models::{CanonicalAssetModel, PolicyModel, PortfolioModel},
    services::MarketPriceUpdate,
};

/// Structured, machine-readable reasons why an allocation proposal was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyRejectionReason {
    /// Asset is marked inactive in the registry.
    AssetInactive { asset_id: String, symbol: String },

    /// Asset fails Shariah compliance (status not Approved, review expired, or impermissible business).
    ShariahNonCompliant {
        asset_id: String,
        symbol: String,
        details: String,
    },

    /// Asset custodial backing and legal ownership structure is unverified.
    AssetOwnershipUnverified {
        asset_id: String,
        symbol: String,
        issuer: String,
    },

    /// Portfolio valuation or global portfolio constraint was breached.
    PortfolioLimitExceeded {
        reason: String,
        current_bps: u16,
        max_bps: u16,
    },

    /// Projected position exposure exceeds vault policy limit (*Bay' al-Gharar* / concentration risk).
    PositionLimitExceeded {
        symbol: String,
        projected_bps: u16,
        max_bps: u16,
    },

    /// Single trade size exceeds maximum allowed capital outlay.
    MaxTradeSizeExceeded { trade_usd: u64, max_trade_usd: u64 },

    /// Insufficient asset balance for SELL trade (*Bay' ma la Yamlik* prohibition).
    InsufficientAssetBalance {
        symbol: String,
        required_amount: u64,
        available_amount: u64,
    },

    /// Insufficient available cash for BUY trade.
    InsufficientCashBalance {
        required_cash_usd: u64,
        available_cash_usd: u64,
    },

    /// Trade breaches minimum cash reserve, violating spot solvency (no leverage allowed).
    SolvencyBreach {
        available_cash_usd: u64,
        required_outflow_usd: u64,
        min_reserve_usd: u64,
    },

    /// Pyth reference price is stale, missing, or future-skewed.
    OraclePriceStale {
        symbol: String,
        publish_time: i64,
        max_staleness_secs: i64,
        age_secs: i64,
    },

    /// Pyth oracle confidence interval is too wide for safe valuation.
    OracleConfidenceTooWide {
        symbol: String,
        conf_bps: u16,
        max_conf_bps: u16,
    },

    /// Executable DEX quote is missing, expired, or mismatches trade mints.
    DexQuoteInvalid { reason: String },

    /// Requested slippage tolerance exceeds vault policy maximum.
    SlippageExceeded {
        requested_bps: u16,
        max_allowed_bps: u16,
    },

    /// DEX quote price impact exceeds policy risk limit.
    PriceImpactExceeded {
        actual_impact_bps: u16,
        max_impact_bps: u16,
    },

    /// Structural defect in the allocation proposal payload.
    InvalidProposal { reason: String },
}

/// An unverified trade allocation proposal.
///
/// May originate from an AI decision model, signal generator, or human portfolio manager.
/// Invariant: An allocation proposal is strictly UNTRUSTED. No field (including any
/// AI-generated `approved=true` or metadata) can bypass deterministic validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationProposal {
    /// Unique identifier for the proposal
    pub proposal_id: Uuid,
    /// Target vault address
    pub vault_address: String,
    /// Target asset canonical identifier (e.g. "backed:AAPLx")
    pub asset_id: String,
    /// Ticker symbol (e.g. "AAPL")
    pub symbol: String,
    /// SPL token mint address
    pub mint_address: String,
    /// Trade direction: true for BUY, false for SELL
    pub is_buy: bool,
    /// Target amount in integer atomic units
    pub target_amount: u64,
    /// Proposed trade value in USD base units
    pub proposed_usd_value: u64,
    /// Maximum acceptable slippage tolerance in basis points
    pub max_slippage_bps: u16,
    /// Proposal creation timestamp
    pub proposed_at: DateTime<Utc>,
    /// Optional AI reasoning / confidence metadata (strictly informational; never trusted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_rationale: Option<String>,
    /// Even if an AI attempts to declare `ai_approved = true`, it is ignored by the validator
    #[serde(default)]
    pub ai_approved: bool,
}

/// An immutable, deterministically validated execution authorization.
///
/// Invariant: An `ExecutionAuthorization` can ONLY be produced by `DeterministicPolicyAuthorizer`
/// when every single Shariah, policy, balance, oracle, and DEX risk rule passes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionAuthorization {
    /// Unique authorization tracking identifier
    pub authorization_id: Uuid,
    /// Referencing proposal identifier
    pub proposal_id: Uuid,
    /// Target vault address
    pub vault_address: String,
    /// Canonical asset identity
    pub asset_id: String,
    /// Asset symbol
    pub symbol: String,
    /// SPL token mint address
    pub mint_address: String,
    /// Trade direction: true for BUY, false for SELL
    pub is_buy: bool,
    /// Authorized amount in integer atomic units
    pub authorized_amount: u64,
    /// Authorized value in USD base units
    pub authorized_usd_value: u64,
    /// Bound slippage tolerance in basis points
    pub max_slippage_bps: u16,

    /// Reference Pyth oracle price scaled in micro-USD at authorization time
    pub oracle_reference_price_scaled: u64,
    /// Pyth oracle publication timestamp
    pub oracle_publish_time: i64,

    /// Executable DEX quote output expectation
    pub dex_expected_output_amount: u64,
    /// Minimum guaranteed output amount from DEX quote
    pub dex_minimum_output_amount: u64,
    /// Executable DEX quote price impact in basis points
    pub dex_price_impact_bps: u16,

    /// Precise evaluation and authorization timestamp
    pub authorized_at: DateTime<Utc>,
    /// Strict expiration timestamp for this authorization (bound by DEX quote expiry)
    pub valid_until: DateTime<Utc>,
    /// Cryptographic / deterministic audit fingerprint of all validated parameters
    pub audit_digest: String,
}

impl ExecutionAuthorization {
    /// Calculates a deterministic SHA-256 fingerprint over all authorized parameters.
    pub fn compute_audit_digest(
        proposal_id: &Uuid,
        vault_address: &str,
        asset_id: &str,
        amount: u64,
        usd_value: u64,
        oracle_price_scaled: u64,
        dex_min_out: u64,
        authorized_at_ts: i64,
        valid_until_ts: i64,
    ) -> String {
        let mut bytes = Vec::with_capacity(256);
        bytes.extend_from_slice(b"EQUITY_CATALYST_AUTHORIZATION_V1");
        bytes.extend_from_slice(proposal_id.as_bytes());
        bytes.extend_from_slice(vault_address.as_bytes());
        bytes.extend_from_slice(asset_id.as_bytes());
        bytes.extend_from_slice(&amount.to_le_bytes());
        bytes.extend_from_slice(&usd_value.to_le_bytes());
        bytes.extend_from_slice(&oracle_price_scaled.to_le_bytes());
        bytes.extend_from_slice(&dex_min_out.to_le_bytes());
        bytes.extend_from_slice(&authorized_at_ts.to_le_bytes());
        bytes.extend_from_slice(&valid_until_ts.to_le_bytes());
        hash(&bytes).to_string()
    }
}

/// The outcome of evaluating an allocation proposal against deterministic rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyValidationOutcome {
    /// Every deterministic check passed; execution authorization issued.
    Authorized(ExecutionAuthorization),
    /// One or more deterministic checks failed; trade proposal rejected.
    Rejected {
        proposal_id: Uuid,
        reasons: Vec<PolicyRejectionReason>,
        evaluated_at: DateTime<Utc>,
    },
}

impl PolicyValidationOutcome {
    pub fn is_authorized(&self) -> bool {
        matches!(self, PolicyValidationOutcome::Authorized(_))
    }

    pub fn authorization(&self) -> Option<&ExecutionAuthorization> {
        match self {
            PolicyValidationOutcome::Authorized(auth) => Some(auth),
            _ => None,
        }
    }

    pub fn rejection_reasons(&self) -> Option<&[PolicyRejectionReason]> {
        match self {
            PolicyValidationOutcome::Rejected { reasons, .. } => Some(reasons),
            _ => None,
        }
    }
}

/// Comprehensive contextual inputs required for deterministic policy and risk authorization.
#[derive(Debug, Clone)]
pub struct ValidationContext<'a> {
    /// Active vault policy rules and limits
    pub policy: &'a PolicyModel,
    /// Existing portfolio positions held in the vault
    pub positions: &'a [PortfolioModel],
    /// Total portfolio valuation in micro-USD or USD base units
    pub total_portfolio_usd: u64,
    /// Available unallocated cash in USD base units
    pub available_cash_usd: u64,
    /// Canonical asset registry record (mandatory for non-cash assets)
    pub canonical_asset: Option<&'a CanonicalAssetModel>,
    /// Shariah registry compliance record (mandatory for non-cash assets)
    pub shariah_record: Option<&'a RegisteredAsset>,
    /// Optional Shariah registry reference for automatic lookup
    pub shariah_registry: Option<&'a ShariahAssetRegistry>,
    /// Validated Pyth reference oracle price (mandatory for non-cash assets)
    pub oracle_price: Option<&'a MarketPriceUpdate>,
    /// Executable DEX quote from DexQuoter (mandatory for execution authorization)
    pub dex_quote: Option<&'a DexQuote>,
    /// Deterministic evaluation timestamp (guarantees identical output for identical time)
    pub evaluation_time: DateTime<Utc>,
    /// Custom maximum trade size limit in USD (if not in PolicyModel, default: 25% of portfolio)
    pub max_trade_size_usd: Option<u64>,
    /// Maximum allowable oracle confidence interval in basis points (default: 50 bps)
    pub max_oracle_conf_bps: Option<u16>,
    /// Maximum allowable slippage in basis points (default: 100 bps = 1.00%)
    pub max_allowed_slippage_bps: Option<u16>,
    /// Maximum allowable DEX price impact in basis points (default: 100 bps = 1.00%)
    pub max_allowed_price_impact_bps: Option<u16>,
    /// Maximum oracle staleness in seconds (default: 30 seconds)
    pub max_oracle_staleness_secs: Option<i64>,
}

/// Deterministic policy and risk authorizer.
///
/// Strictly transforms an untrusted `AllocationProposal` into an `ExecutionAuthorization`
/// ONLY when all 13 deterministic criteria pass.
#[derive(Debug, Clone, Default)]
pub struct DeterministicPolicyAuthorizer;

impl DeterministicPolicyAuthorizer {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates an allocation proposal against all deterministic rules.
    ///
    /// Evaluates:
    /// 1. Asset active check
    /// 2. Asset Shariah-approved check
    /// 3. Asset ownership valid check
    /// 4. Portfolio limits check
    /// 5. Position limits check
    /// 6. Maximum trade size check
    /// 7. Available balance check (asset oversell or cash sufficiency)
    /// 8. Solvency check (cash reserve retention without leverage)
    /// 9. Oracle freshness check
    /// 10. Oracle confidence check
    /// 11. DEX quote validity check
    /// 12. Slippage check
    /// 13. Price impact check
    pub fn authorize(
        &self,
        proposal: &AllocationProposal,
        context: &ValidationContext,
    ) -> PolicyValidationOutcome {
        let mut reasons = Vec::new();

        // Basic structural validation
        if proposal.target_amount == 0 || proposal.proposed_usd_value == 0 {
            reasons.push(PolicyRejectionReason::InvalidProposal {
                reason: "Trade amount and proposed USD value must be greater than zero".to_string(),
            });
        }

        let is_cash = proposal.symbol.eq_ignore_ascii_case("USDC")
            || proposal.symbol.eq_ignore_ascii_case("CASH");

        // 1. Asset Active Check
        if !is_cash {
            match context.canonical_asset {
                Some(asset) => {
                    if !asset.is_active {
                        reasons.push(PolicyRejectionReason::AssetInactive {
                            asset_id: proposal.asset_id.clone(),
                            symbol: proposal.symbol.clone(),
                        });
                    }
                }
                None => {
                    reasons.push(PolicyRejectionReason::AssetInactive {
                        asset_id: proposal.asset_id.clone(),
                        symbol: proposal.symbol.clone(),
                    });
                }
            }
        }

        // Resolve Shariah record (explicit or from registry)
        let resolved_shariah = context.shariah_record.or_else(|| {
            context
                .shariah_registry
                .and_then(|reg| reg.get_asset(&proposal.symbol))
        });

        // 2. Asset Shariah-Approved Check
        if !is_cash {
            let now_ts = context.evaluation_time.timestamp();
            match resolved_shariah {
                Some(record) => {
                    let elig = &record.eligibility;
                    if elig.status != ShariahStatus::Approved {
                        reasons.push(PolicyRejectionReason::ShariahNonCompliant {
                            asset_id: proposal.asset_id.clone(),
                            symbol: proposal.symbol.clone(),
                            details: format!("Status is {} (must be Approved)", elig.status),
                        });
                    } else if !elig.business_activity_approved {
                        reasons.push(PolicyRejectionReason::ShariahNonCompliant {
                            asset_id: proposal.asset_id.clone(),
                            symbol: proposal.symbol.clone(),
                            details: "Core business activity is not permissible".to_string(),
                        });
                    } else if now_ts < elig.reviewed_at || now_ts >= elig.expires_at {
                        reasons.push(PolicyRejectionReason::ShariahNonCompliant {
                            asset_id: proposal.asset_id.clone(),
                            symbol: proposal.symbol.clone(),
                            details:
                                "Screening review has expired or evaluation timestamp is invalid"
                                    .to_string(),
                        });
                    }
                }
                None => {
                    reasons.push(PolicyRejectionReason::ShariahNonCompliant {
                        asset_id: proposal.asset_id.clone(),
                        symbol: proposal.symbol.clone(),
                        details: "Asset is not registered in ShariahAssetRegistry".to_string(),
                    });
                }
            }
        }

        // 3. Asset Ownership Valid Check
        if !is_cash {
            match resolved_shariah {
                Some(record) => {
                    if !record.eligibility.ownership_verified {
                        let issuer = context
                            .canonical_asset
                            .map(|a| a.legal_issuer.clone())
                            .unwrap_or_else(|| record.asset.provider.provider.to_string());
                        reasons.push(PolicyRejectionReason::AssetOwnershipUnverified {
                            asset_id: proposal.asset_id.clone(),
                            symbol: proposal.symbol.clone(),
                            issuer,
                        });
                    }
                }
                None => {
                    // Already flagged by Shariah check
                }
            }
        }

        // 4. Portfolio Limits Check
        if context.total_portfolio_usd == 0 {
            reasons.push(PolicyRejectionReason::PortfolioLimitExceeded {
                reason: "Total portfolio valuation cannot be zero".to_string(),
                current_bps: 0,
                max_bps: 10_000,
            });
        }

        // 5. Position Limits Check (for BUY trades)
        if proposal.is_buy && context.total_portfolio_usd > 0 {
            let current_pos_usd = context
                .positions
                .iter()
                .find(|p| p.asset_symbol.eq_ignore_ascii_case(&proposal.symbol))
                .map(|p| p.current_value_usd as u64)
                .unwrap_or(0);

            let projected_pos_usd = current_pos_usd.saturating_add(proposal.proposed_usd_value);
            let projected_bps =
                ((projected_pos_usd as u128 * 10_000) / context.total_portfolio_usd as u128) as u16;
            let max_pos_bps = context.policy.max_position_bps as u16;

            if projected_bps > max_pos_bps {
                reasons.push(PolicyRejectionReason::PositionLimitExceeded {
                    symbol: proposal.symbol.clone(),
                    projected_bps,
                    max_bps: max_pos_bps,
                });
            }
        }

        // 6. Maximum Trade Size Check
        let max_trade_size = context
            .max_trade_size_usd
            .unwrap_or_else(|| (context.total_portfolio_usd * 25) / 100); // 25% default limit

        if proposal.proposed_usd_value > max_trade_size {
            reasons.push(PolicyRejectionReason::MaxTradeSizeExceeded {
                trade_usd: proposal.proposed_usd_value,
                max_trade_usd: max_trade_size,
            });
        }

        // 7. Available Balance Check
        if proposal.is_buy {
            if context.available_cash_usd < proposal.proposed_usd_value {
                reasons.push(PolicyRejectionReason::InsufficientCashBalance {
                    required_cash_usd: proposal.proposed_usd_value,
                    available_cash_usd: context.available_cash_usd,
                });
            }
        } else {
            // SELL trade: check asset balance
            let current_pos = context
                .positions
                .iter()
                .find(|p| p.asset_symbol.eq_ignore_ascii_case(&proposal.symbol));

            let available_amount = current_pos.map(|p| p.amount).unwrap_or(0);
            if available_amount < proposal.target_amount || available_amount == 0 {
                reasons.push(PolicyRejectionReason::InsufficientAssetBalance {
                    symbol: proposal.symbol.clone(),
                    required_amount: proposal.target_amount,
                    available_amount,
                });
            }
        }

        // 8. Solvency Check (for BUY trades: minimum cash reserve required post-trade)
        if proposal.is_buy && context.total_portfolio_usd > 0 {
            let min_cash_bps = context.policy.min_cash_bps as u16;
            let min_reserve_usd =
                ((context.total_portfolio_usd as u128 * min_cash_bps as u128) / 10_000) as u64;

            let remaining_cash = context
                .available_cash_usd
                .saturating_sub(proposal.proposed_usd_value);

            if context.available_cash_usd < proposal.proposed_usd_value
                || remaining_cash < min_reserve_usd
            {
                reasons.push(PolicyRejectionReason::SolvencyBreach {
                    available_cash_usd: context.available_cash_usd,
                    required_outflow_usd: proposal.proposed_usd_value,
                    min_reserve_usd,
                });
            }
        }

        // 9. Oracle Freshness Check
        let now_ts = context.evaluation_time.timestamp();
        let max_staleness = context.max_oracle_staleness_secs.unwrap_or(30);

        match context.oracle_price {
            Some(oracle) => {
                let age = now_ts - oracle.publish_time;
                if age > max_staleness || age < -5 {
                    reasons.push(PolicyRejectionReason::OraclePriceStale {
                        symbol: proposal.symbol.clone(),
                        publish_time: oracle.publish_time,
                        max_staleness_secs: max_staleness,
                        age_secs: age,
                    });
                }
            }
            None => {
                if !is_cash {
                    reasons.push(PolicyRejectionReason::OraclePriceStale {
                        symbol: proposal.symbol.clone(),
                        publish_time: 0,
                        max_staleness_secs: max_staleness,
                        age_secs: i64::MAX,
                    });
                }
            }
        }

        // 10. Oracle Confidence Check
        let max_conf_bps = context.max_oracle_conf_bps.unwrap_or(50);
        if let Some(oracle) = context.oracle_price {
            if oracle.conf_bps > max_conf_bps {
                reasons.push(PolicyRejectionReason::OracleConfidenceTooWide {
                    symbol: proposal.symbol.clone(),
                    conf_bps: oracle.conf_bps,
                    max_conf_bps,
                });
            }
        }

        // 11. DEX Quote Validity Check
        match context.dex_quote {
            Some(quote) => {
                if quote.is_expired(now_ts) {
                    reasons.push(PolicyRejectionReason::DexQuoteInvalid {
                        reason: format!(
                            "DEX quote expired at {} (current time: {})",
                            quote.expires_at, now_ts
                        ),
                    });
                } else if !proposal.is_buy && quote.input_mint != proposal.mint_address {
                    reasons.push(PolicyRejectionReason::DexQuoteInvalid {
                        reason: "DEX quote input mint does not match sell asset mint".to_string(),
                    });
                } else if proposal.is_buy && quote.output_mint != proposal.mint_address {
                    reasons.push(PolicyRejectionReason::DexQuoteInvalid {
                        reason: "DEX quote output mint does not match buy asset mint".to_string(),
                    });
                }
            }
            None => {
                reasons.push(PolicyRejectionReason::DexQuoteInvalid {
                    reason: "Executable DEX quote is missing".to_string(),
                });
            }
        }

        // 12. Slippage Check
        let max_slippage_allowed = context.max_allowed_slippage_bps.unwrap_or(100); // 1.00%
        if proposal.max_slippage_bps > max_slippage_allowed {
            reasons.push(PolicyRejectionReason::SlippageExceeded {
                requested_bps: proposal.max_slippage_bps,
                max_allowed_bps: max_slippage_allowed,
            });
        }

        // 13. Price Impact Check
        let max_impact_allowed = context.max_allowed_price_impact_bps.unwrap_or(100); // 1.00%
        if let Some(quote) = context.dex_quote {
            if quote.price_impact_bps > max_impact_allowed {
                reasons.push(PolicyRejectionReason::PriceImpactExceeded {
                    actual_impact_bps: quote.price_impact_bps,
                    max_impact_bps: max_impact_allowed,
                });
            }
        }

        // DECISION DETERMINATION
        if !reasons.is_empty() {
            PolicyValidationOutcome::Rejected {
                proposal_id: proposal.proposal_id,
                reasons,
                evaluated_at: context.evaluation_time,
            }
        } else {
            let quote = context.dex_quote.unwrap();
            let oracle_scaled = context
                .oracle_price
                .map(|p| p.price_scaled)
                .unwrap_or(1_000_000); // 1.00 USD for cash
            let oracle_time = context
                .oracle_price
                .map(|p| p.publish_time)
                .unwrap_or_else(|| context.evaluation_time.timestamp());

            let valid_until_ts = quote.expires_at;
            let valid_until = DateTime::from_timestamp(valid_until_ts, 0)
                .unwrap_or_else(|| context.evaluation_time);

            let audit_digest = ExecutionAuthorization::compute_audit_digest(
                &proposal.proposal_id,
                &proposal.vault_address,
                &proposal.asset_id,
                proposal.target_amount,
                proposal.proposed_usd_value,
                oracle_scaled,
                quote.minimum_output_amount,
                context.evaluation_time.timestamp(),
                valid_until_ts,
            );

            PolicyValidationOutcome::Authorized(ExecutionAuthorization {
                authorization_id: Uuid::new_v4(),
                proposal_id: proposal.proposal_id,
                vault_address: proposal.vault_address.clone(),
                asset_id: proposal.asset_id.clone(),
                symbol: proposal.symbol.clone(),
                mint_address: proposal.mint_address.clone(),
                is_buy: proposal.is_buy,
                authorized_amount: proposal.target_amount,
                authorized_usd_value: proposal.proposed_usd_value,
                max_slippage_bps: proposal.max_slippage_bps,
                oracle_reference_price_scaled: oracle_scaled,
                oracle_publish_time: oracle_time,
                dex_expected_output_amount: quote.expected_output_amount,
                dex_minimum_output_amount: quote.minimum_output_amount,
                dex_price_impact_bps: quote.price_impact_bps,
                authorized_at: context.evaluation_time,
                valid_until,
                audit_digest,
            })
        }
    }
}
