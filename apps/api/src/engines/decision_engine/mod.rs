//! Decision Engine coordinating PolicyEngine, ShariahGate, RiskEngine, and ExecutionSigner into ExecutionRequests.

pub mod decision;
pub mod signer;
pub mod validation;

pub use decision::{ExecutionRequest, TradeOrder};
pub use signer::ExecutionSigner;
pub use validation::validate_decision_preflight;

use chrono::Utc;
use equity_catalyst_shared::{
    asset::verified_mainnet_assets,
    provider::{ProviderResolver, ResolutionKind},
    shariah::{
        BusinessCategory, OwnershipRecord, ScreeningPolicy, ShariahAssetRegistry,
        ShariahFinancialMetrics, ShariahStatus,
    },
    types::SignalType,
};
use uuid::Uuid;

use crate::{
    engines::{
        policy_engine::PolicyEngine,
        risk_engine::{RiskAssessment, RiskEngine},
    },
    error::ApiError,
    models::{EventModel, PolicyModel, PortfolioModel, VaultModel},
};

/// Deterministic result of ShariahGate compliance verification before trade execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShariahGateAssessment {
    Approved,
    Rejected { symbol: String, reason: String },
}

// ============================================================================
// TEST FIXTURE DATA
// ============================================================================
// Synthetic test fixtures used for deterministic unit and integration tests.
//
// WARNING: Strings such as "test_fixture:cert_backed_nvda" are synthetic test
// identifiers and MUST NEVER be treated or described as real cryptographic hashes.
//
// ============================================================================
// PRODUCTION VERIFIED DATA
// ============================================================================
// Production compliance requires genuine SHA-256 cryptographic digests of
// verified statutory ownership certificates and audited corporate financial
// filings supplied through the authoritative verification pipeline.

/// Builds the deterministic test fixture Shariah registry for local tests and development.
pub fn test_fixture_shariah_registry() -> ShariahAssetRegistry {
    let mut registry = ShariahAssetRegistry::new();
    let policy = ScreeningPolicy::board_approved_v1();
    let verified = verified_mainnet_assets();

    let base_verified_at = 1_700_000_000;
    let base_expires_at = 2_000_000_000; // Audited window valid through year 2033

    for asset in verified {
        // TEST FIXTURE DATA: Synthetic financial ratios and mock fixture hashes for test execution.
        let (business, debt, cash, impure, cert_hash) = match asset.symbol() {
            "NVDA" => (
                BusinessCategory::Technology,
                1_500, // 15.00% <= 30.00%
                1_200, // 12.00% <= 30.00%
                100,   // 1.00% <= 5.00%
                "test_fixture_hash:cert_backed_nvda",
            ),
            "AAPL" => (
                BusinessCategory::Technology,
                2_100, // 21.00% <= 30.00%
                1_400, // 14.00% <= 30.00%
                120,   // 1.20% <= 5.00%
                "test_fixture_hash:cert_backed_aapl",
            ),
            "SPYx" => (
                BusinessCategory::Manufacturing,
                2_500, // 25.00% <= 30.00%
                2_000, // 20.00% <= 30.00%
                200,   // 2.00% <= 5.00%
                "test_fixture_hash:cert_backed_spyx",
            ),
            _ => continue,
        };

        let ownership = OwnershipRecord {
            verified: true,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: format!("ISIN: {}", asset.identity.underlying_reference),
            evidence_hash: cert_hash.to_string(),
            verified_at: base_verified_at,
            expires_at: base_expires_at,
        };

        let financials = ShariahFinancialMetrics {
            debt_ratio_bps: debt,
            interest_bearing_cash_ratio_bps: cash,
            impure_income_ratio_bps: impure,
        };

        let _ = registry.register(
            asset,
            ownership,
            financials,
            business,
            &policy,
            base_verified_at,
        );
    }

    registry
}

/// Convenience alias referencing `test_fixture_shariah_registry` for development and test harnesses.
/// For production environments, inject `production_shariah_registry` populated with verified cryptographic evidence.
pub fn canonical_shariah_registry() -> ShariahAssetRegistry {
    test_fixture_shariah_registry()
}

#[derive(Clone, Debug)]
pub struct DecisionEngine {
    policy_engine: PolicyEngine,
    risk_engine: RiskEngine,
    signer: ExecutionSigner,
    shariah_registry: ShariahAssetRegistry,
}

impl DecisionEngine {
    pub fn new(signer: ExecutionSigner) -> Self {
        Self {
            policy_engine: PolicyEngine::new(),
            risk_engine: RiskEngine::new(),
            signer,
            shariah_registry: canonical_shariah_registry(),
        }
    }

    /// Builder method allowing dependency injection of a custom Shariah registry for testing.
    pub fn with_registry(mut self, registry: ShariahAssetRegistry) -> Self {
        self.shariah_registry = registry;
        self
    }

    pub fn signer(&self) -> &ExecutionSigner {
        &self.signer
    }

    pub fn registry(&self) -> &ShariahAssetRegistry {
        &self.shariah_registry
    }

    pub fn registry_mut(&mut self) -> &mut ShariahAssetRegistry {
        &mut self.shariah_registry
    }

    /// Evaluates proposed trades against the 7 mandatory ShariahGate requirements:
    /// 1. Resolve asset identity
    /// 2. Retrieve Shariah eligibility
    /// 3. Require Approved status
    /// 4. Require ownership_verified == true
    /// 5. Require review not expired (now >= reviewed_at and now < expires_at)
    /// 6. Require approved business category (non-prohibited)
    /// 7. Require current screening policy version
    pub fn evaluate_shariah_gate(
        &self,
        trades: &[TradeOrder],
        _policy: &PolicyModel,
        now: i64,
    ) -> ShariahGateAssessment {
        for trade in trades {
            let symbol = trade.symbol.trim();
            if symbol.eq_ignore_ascii_case("USDC") || symbol.eq_ignore_ascii_case("CASH") {
                continue;
            }

            // 1. Resolve asset identity via ProviderResolver
            let resolved = match ProviderResolver::resolve(symbol) {
                Some(r) => r,
                None => {
                    return ShariahGateAssessment::Rejected {
                        symbol: symbol.to_string(),
                        reason: format!(
                            "Asset '{}' could not be resolved by ProviderResolver",
                            symbol
                        ),
                    };
                }
            };

            // Unverified fallback lookups can never enter execution
            if resolved.resolution_kind == ResolutionKind::FallbackIdentity {
                return ShariahGateAssessment::Rejected {
                    symbol: symbol.to_string(),
                    reason: format!(
                        "Asset '{}' resolved via unverified fallback identity only; provider authenticity unverified",
                        symbol
                    ),
                };
            }

            // 2. Retrieve Shariah eligibility from the registry gate
            let registered = match self.shariah_registry.get_asset(symbol) {
                Some(r) => r,
                None => {
                    return ShariahGateAssessment::Rejected {
                        symbol: symbol.to_string(),
                        reason: format!(
                            "Asset '{}' is not registered in ShariahAssetRegistry",
                            symbol
                        ),
                    };
                }
            };

            let eligibility = &registered.eligibility;

            // 3. Require Approved status
            if eligibility.status != ShariahStatus::Approved {
                return ShariahGateAssessment::Rejected {
                    symbol: symbol.to_string(),
                    reason: format!(
                        "Asset '{}' Shariah status is {} (must be Approved)",
                        symbol, eligibility.status
                    ),
                };
            }

            // 4. Require ownership_verified
            if !eligibility.ownership_verified {
                return ShariahGateAssessment::Rejected {
                    symbol: symbol.to_string(),
                    reason: format!("Asset '{}' custodial ownership is unverified", symbol),
                };
            }

            // 5. Require review not expired
            if now < eligibility.reviewed_at || now >= eligibility.expires_at {
                return ShariahGateAssessment::Rejected {
                    symbol: symbol.to_string(),
                    reason: format!(
                        "Asset '{}' Shariah review has expired or evaluation timestamp is invalid",
                        symbol
                    ),
                };
            }

            // 6. Require approved business category
            if !eligibility.business_activity_approved {
                return ShariahGateAssessment::Rejected {
                    symbol: symbol.to_string(),
                    reason: format!(
                        "Asset '{}' core business activity is not permissible",
                        symbol
                    ),
                };
            }

            // 7. Require current screening policy version
            let current_policy_version = "v1.0";
            if eligibility.policy_version != current_policy_version {
                return ShariahGateAssessment::Rejected {
                    symbol: symbol.to_string(),
                    reason: format!(
                        "Asset '{}' screening policy version mismatch (expected '{}', got '{}')",
                        symbol, current_policy_version, eligibility.policy_version
                    ),
                };
            }
        }

        ShariahGateAssessment::Approved
    }

    /// Complete decision pipeline:
    /// Event -> Policy Engine -> ShariahGate -> Risk Engine -> Decision Engine -> Execution Request
    pub fn process_event(
        &self,
        event: &EventModel,
        vault: &VaultModel,
        policy: &PolicyModel,
        positions: &[PortfolioModel],
        total_value_usd: u64,
        available_cash_usd: u64,
    ) -> Result<ExecutionRequest, ApiError> {
        let decision_id = Uuid::new_v4();

        // 1. Policy Engine evaluation
        let policy_result =
            self.policy_engine
                .evaluate(event, policy, positions, total_value_usd)?;

        // 2. Map RebalanceTrade to TradeOrders
        let trade_orders: Vec<TradeOrder> = policy_result
            .proposed_trades
            .iter()
            .map(|t| TradeOrder {
                symbol: t.symbol.clone(),
                is_buy: t.is_buy,
                usd_value: t.trade_value,
            })
            .collect();

        // 3. ShariahGate: Verify asset qualification before risk evaluation or trade execution
        let now = Utc::now().timestamp();
        let shariah_assessment = self.evaluate_shariah_gate(&trade_orders, policy, now);

        let (approved, rationale) = match shariah_assessment {
            ShariahGateAssessment::Rejected { symbol, reason } => (
                false,
                format!(
                    "Rejected by ShariahGate for symbol '{}': {}. AI reasoning was informational only.",
                    symbol, reason
                ),
            ),
            ShariahGateAssessment::Approved => {
                // 4. Risk Engine assessment (only reached if ShariahGate approves)
                let risk_assessment = self.risk_engine.evaluate_proposed_trades(
                    &policy_result.proposed_trades,
                    positions,
                    policy,
                    total_value_usd,
                    available_cash_usd,
                );

                match risk_assessment {
                    RiskAssessment::Approved => (
                        true,
                        format!(
                            "Approved by Risk Engine. Policy rationale: {}",
                            policy_result.signal.reason
                        ),
                    ),
                    RiskAssessment::Rejected { reason } => {
                        (false, format!("Rejected by Risk Engine: {}", reason))
                    }
                }
            }
        };

        let action = match policy_result.signal.signal_type {
            SignalType::EmergencyExit => "EMERGENCY_EXIT",
            SignalType::Bullish => "BUY",
            SignalType::Bearish => "SELL",
            SignalType::RebalanceRequired => "REBALANCE",
            SignalType::Neutral => "NOOP",
        }
        .to_string();

        let request = ExecutionRequest {
            decision_id,
            vault_address: vault.vault_address.clone(),
            event_id: Some(event.event_id),
            action,
            trades: trade_orders,
            approved,
            rationale,
            timestamp: Utc::now(),
        };

        // 5. Pre-flight decision validation
        if let Err(err) = validate_decision_preflight(vault, policy, &request) {
            return Err(ApiError::BadRequest(err));
        }

        Ok(request)
    }
}
