//! Decision Engine coordinating PolicyEngine, RiskEngine, and ExecutionSigner into ExecutionRequests.

pub mod decision;
pub mod signer;
pub mod validation;

pub use decision::{ExecutionRequest, TradeOrder};
pub use signer::ExecutionSigner;
pub use validation::validate_decision_preflight;

use chrono::Utc;
use equity_catalyst_shared::types::SignalType;
use uuid::Uuid;

use crate::{
    engines::{
        policy_engine::PolicyEngine,
        risk_engine::{RiskAssessment, RiskEngine},
    },
    error::ApiError,
    models::{EventModel, PolicyModel, PortfolioModel, VaultModel},
};

#[derive(Clone, Debug)]
pub struct DecisionEngine {
    policy_engine: PolicyEngine,
    risk_engine: RiskEngine,
    signer: ExecutionSigner,
}

impl DecisionEngine {
    pub fn new(signer: ExecutionSigner) -> Self {
        Self {
            policy_engine: PolicyEngine::new(),
            risk_engine: RiskEngine::new(),
            signer,
        }
    }

    pub fn signer(&self) -> &ExecutionSigner {
        &self.signer
    }

    /// Complete decision pipeline:
    /// Event -> Policy Engine -> Risk Engine -> Decision Engine -> Execution Request
    pub fn process_event(
        &self,
        event: &EventModel,
        vault: &VaultModel,
        policy: &PolicyModel,
        positions: &[PortfolioModel],
        total_value_usd: u64,
        total_debt_usd: u64,
    ) -> Result<ExecutionRequest, ApiError> {
        let decision_id = Uuid::new_v4();

        // 1. Policy Engine evaluation
        let policy_result = self.policy_engine.evaluate(
            event,
            policy,
            positions,
            total_value_usd,
        )?;

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

        // 3. Risk Engine assessment
        let risk_assessment = self.risk_engine.evaluate_proposed_trades(
            &policy_result.proposed_trades,
            positions,
            policy,
            total_value_usd,
            total_debt_usd,
        );

        let (approved, rationale) = match risk_assessment {
            RiskAssessment::Approved => (
                true,
                format!(
                    "Approved by Risk Engine. Policy rationale: {}",
                    policy_result.signal.reason
                ),
            ),
            RiskAssessment::Rejected { reason } => (
                false,
                format!("Rejected by Risk Engine: {}", reason),
            ),
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

        // 4. Pre-flight decision validation
        if let Err(err) = validate_decision_preflight(vault, policy, &request) {
            return Err(ApiError::BadRequest(err));
        }

        Ok(request)
    }
}
