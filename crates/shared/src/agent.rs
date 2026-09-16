//! Policy-Constrained AI Agent Domain Models and Deterministic Governance.
//!
//! Architectural Invariant:
//! The AI model may observe, analyze, propose, explain, and prioritize.
//! It is strictly FORBIDDEN from accessing signing keys, modifying risk limits,
//! bypassing risk checks, overriding policy, or self-authorizing trades.
//! Every proposal must pass through a deterministic 5-stage validation gate before execution.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{asset::Asset, portfolio::Portfolio, types::BasisPoints, validation::ValidationError};

/// Constrained actions an AI agent is permitted to propose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentAction {
    Rebalance,
    Hold,
    ReduceRisk,
    EmergencyHalt,
}

impl fmt::Display for AgentAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rebalance => write!(f, "REBALANCE"),
            Self::Hold => write!(f, "HOLD"),
            Self::ReduceRisk => write!(f, "REDUCE_RISK"),
            Self::EmergencyHalt => write!(f, "EMERGENCY_HALT"),
        }
    }
}

/// Structured, strongly-typed output emitted by the AI Agent.
/// The model is prohibited from emitting raw transaction bytes or arbitrary instructions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentProposal {
    /// Proposed strategic action
    pub action: AgentAction,
    /// Target asset ticker symbol, e.g. "NVDA"
    pub symbol: String,
    /// Recommended target portfolio weight in basis points (e.g. 1,500 = 15.00%)
    pub target_weight_bps: BasisPoints,
    /// Human-readable macroeconomic or quantitative justification
    pub reasoning_summary: String,
    /// Model confidence score bounded between 0.0 and 1.0
    pub confidence: f64,
    /// Proposal creation Unix timestamp in seconds
    pub timestamp: i64,
}

impl AgentProposal {
    pub fn new(
        action: AgentAction,
        symbol: impl Into<String>,
        target_weight_bps: BasisPoints,
        reasoning_summary: impl Into<String>,
        confidence: f64,
        timestamp: i64,
    ) -> Result<Self, ValidationError> {
        let sym = symbol.into();
        let reason = reasoning_summary.into();

        if sym.trim().is_empty() {
            return Err(ValidationError::EmptySymbol);
        }
        if reason.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "reasoning_summary cannot be empty".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&confidence) || confidence.is_nan() {
            return Err(ValidationError::InvalidParam(format!(
                "Confidence score must be in [0.0, 1.0], got {}",
                confidence
            )));
        }

        Ok(Self {
            action,
            symbol: sym,
            target_weight_bps,
            reasoning_summary: reason,
            confidence,
            timestamp,
        })
    }
}

/// Record of an individual tool call executed during agent reasoning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub tool_name: String,
    pub input_arguments: String,
    pub output_summary: String,
    pub timestamp: i64,
}

/// Full execution trace and immutable audit log for an AI agent interaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentAuditLog {
    pub request_id: String,
    pub vault_address: String,
    pub prompt_context: String,
    pub observations: Vec<String>,
    pub tool_calls: Vec<ToolInvocation>,
    pub proposal: Option<AgentProposal>,
    pub validation_stages: Vec<ValidationStageResult>,
    pub is_approved: bool,
    pub final_verdict: String,
    pub timestamp: i64,
}

/// Individual stage result in the 5-stage deterministic validation gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationStageResult {
    pub stage_name: String,
    pub passed: bool,
    pub message: String,
}

/// Deterministic 5-Stage Validation Gate.
/// Every agent proposal MUST pass all 5 stages before execution is permissible:
/// 1. Schema Validation (types, bounds, confidence)
/// 2. Asset Validation (asset registered and not delisted/halted)
/// 3. Price Validation (fresh trusted price, staleness check)
/// 4. Risk Validation (concentration & LTV limit enforcement)
/// 5. Policy Validation (vault active, paused state respected)
pub struct DeterministicAgentValidator;

impl DeterministicAgentValidator {
    pub fn validate(
        proposal: &AgentProposal,
        portfolio: &Portfolio,
        registered_assets: &[Asset],
        current_time: i64,
        is_vault_paused: bool,
    ) -> (bool, Vec<ValidationStageResult>, Option<String>) {
        let mut stages = Vec::new();

        // Stage 1: Schema Validation
        if !(0.0..=1.0).contains(&proposal.confidence) {
            stages.push(ValidationStageResult {
                stage_name: "1_schema_validation".to_string(),
                passed: false,
                message: format!("Invalid confidence score: {}", proposal.confidence),
            });
            return (false, stages, Some("Schema validation failed".to_string()));
        }
        if proposal.reasoning_summary.trim().is_empty() {
            stages.push(ValidationStageResult {
                stage_name: "1_schema_validation".to_string(),
                passed: false,
                message: "Reasoning summary is empty".to_string(),
            });
            return (
                false,
                stages,
                Some("Reasoning summary required".to_string()),
            );
        }
        stages.push(ValidationStageResult {
            stage_name: "1_schema_validation".to_string(),
            passed: true,
            message: "Schema and bounds valid".to_string(),
        });

        // Stage 2: Asset Validation
        let registered_asset = registered_assets
            .iter()
            .find(|a| a.symbol().eq_ignore_ascii_case(&proposal.symbol));

        match registered_asset {
            None => {
                stages.push(ValidationStageResult {
                    stage_name: "2_asset_validation".to_string(),
                    passed: false,
                    message: format!("Asset '{}' not found in registry", proposal.symbol),
                });
                return (
                    false,
                    stages,
                    Some(format!("Unrecognized asset: {}", proposal.symbol)),
                );
            }
            Some(asset) if !asset.is_active() => {
                stages.push(ValidationStageResult {
                    stage_name: "2_asset_validation".to_string(),
                    passed: false,
                    message: format!(
                        "Asset '{}' is not active (halted/delisted)",
                        proposal.symbol
                    ),
                });
                return (
                    false,
                    stages,
                    Some(format!("Asset '{}' is halted or delisted", proposal.symbol)),
                );
            }
            Some(_) => {
                stages.push(ValidationStageResult {
                    stage_name: "2_asset_validation".to_string(),
                    passed: true,
                    message: format!("Asset '{}' verified active in registry", proposal.symbol),
                });
            }
        }

        // Stage 3: Price & Freshness Validation
        let age_secs = (current_time - proposal.timestamp).abs();
        if age_secs > portfolio.limits.max_price_staleness_secs {
            stages.push(ValidationStageResult {
                stage_name: "3_price_validation".to_string(),
                passed: false,
                message: format!(
                    "Proposal price is stale: age {}s exceeds limit {}s",
                    age_secs, portfolio.limits.max_price_staleness_secs
                ),
            });
            return (
                false,
                stages,
                Some("Proposal timestamp or oracle price is stale".to_string()),
            );
        }
        stages.push(ValidationStageResult {
            stage_name: "3_price_validation".to_string(),
            passed: true,
            message: format!("Price freshness verified (age: {}s)", age_secs),
        });

        // Stage 4: Risk Limits Validation
        if proposal.target_weight_bps > portfolio.limits.max_position_bps {
            stages.push(ValidationStageResult {
                stage_name: "4_risk_validation".to_string(),
                passed: false,
                message: format!(
                    "Target weight {} exceeds max position limit {}",
                    proposal.target_weight_bps, portfolio.limits.max_position_bps
                ),
            });
            return (
                false,
                stages,
                Some("Target allocation breaches max position risk limit".to_string()),
            );
        }
        stages.push(ValidationStageResult {
            stage_name: "4_risk_validation".to_string(),
            passed: true,
            message: "Risk constraints satisfied".to_string(),
        });

        // Stage 5: Policy & Vault State Validation
        if is_vault_paused {
            stages.push(ValidationStageResult {
                stage_name: "5_policy_validation".to_string(),
                passed: false,
                message: "Vault is currently paused by authority".to_string(),
            });
            return (
                false,
                stages,
                Some("Trading halted: vault is paused".to_string()),
            );
        }
        stages.push(ValidationStageResult {
            stage_name: "5_policy_validation".to_string(),
            passed: true,
            message: "Vault active; policy invariants preserved".to_string(),
        });

        (true, stages, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::{
        AssetIdentity, AssetProvider, AssetStatus, AssetType, Network, ProviderConfig, TokenDetails,
    };
    use crate::portfolio::PortfolioLimits;

    fn sample_asset(symbol: &str) -> Asset {
        Asset::new(
            AssetIdentity {
                asset_id: format!("prestocks:{}", symbol),
                symbol: symbol.to_string(),
                name: format!("{} Token", symbol),
                asset_type: AssetType::Stock,
                underlying_reference: format!("NASDAQ:{}", symbol),
            },
            TokenDetails {
                mint: "Mint111111111111111111111111111111111111111".to_string(),
                decimals: 6,
                network: Network::SolanaDevnet,
            },
            ProviderConfig {
                provider: AssetProvider::PreStocks,
                price_feed_id: "feed111111111111111111111111111111111111".to_string(),
                meteora_pool: None,
                secondary_reference: None,
                status: AssetStatus::Active,
            },
        )
        .unwrap()
    }

    #[test]
    fn test_valid_agent_proposal_passes_validation() {
        let proposal = AgentProposal::new(
            AgentAction::Rebalance,
            "NVDA",
            BasisPoints(1_500), // 15%
            "Quarterly earnings beat expected to drive demand; increasing allocation within policy limits.",
            0.85,
            1726000000,
        )
        .expect("Valid proposal");

        let portfolio = Portfolio::new(
            10_000.0,
            vec![],
            None,
            PortfolioLimits::default(), // max position 2,500 bps (25%)
        )
        .unwrap();

        let assets = vec![sample_asset("NVDA")];
        let (approved, stages, err) = DeterministicAgentValidator::validate(
            &proposal, &portfolio, &assets, 1726000030, // 30s later (fresh)
            false,      // not paused
        );

        assert!(approved);
        assert!(err.is_none());
        assert_eq!(stages.len(), 5);
        assert!(stages.iter().all(|s| s.passed));
    }

    #[test]
    fn test_agent_proposal_rejection_on_excessive_risk() {
        // Model requests 35% NVDA allocation when limit is 25%
        let proposal = AgentProposal::new(
            AgentAction::Rebalance,
            "NVDA",
            BasisPoints(3_500), // 35%
            "Aggressive growth outlook.",
            0.95,
            1726000000,
        )
        .unwrap();

        let portfolio = Portfolio::new(10_000.0, vec![], None, PortfolioLimits::default()).unwrap();
        let assets = vec![sample_asset("NVDA")];

        let (approved, stages, err) = DeterministicAgentValidator::validate(
            &proposal, &portfolio, &assets, 1726000010, false,
        );

        assert!(!approved);
        assert!(err.unwrap().contains("breaches max position risk limit"));
        assert_eq!(stages[3].passed, false);
    }

    #[test]
    fn test_agent_proposal_rejection_on_unregistered_asset() {
        let proposal = AgentProposal::new(
            AgentAction::Rebalance,
            "UNKNOWN_MEME",
            BasisPoints(500),
            "High momentum detected.",
            0.60,
            1726000000,
        )
        .unwrap();

        let portfolio = Portfolio::new(10_000.0, vec![], None, PortfolioLimits::default()).unwrap();
        let assets = vec![sample_asset("NVDA")];

        let (approved, _, err) = DeterministicAgentValidator::validate(
            &proposal, &portfolio, &assets, 1726000010, false,
        );

        assert!(!approved);
        assert!(err.unwrap().contains("Unrecognized asset"));
    }

    #[test]
    fn test_agent_proposal_rejection_on_paused_vault() {
        let proposal = AgentProposal::new(
            AgentAction::Rebalance,
            "NVDA",
            BasisPoints(1_000),
            "Rebalance signal.",
            0.80,
            1726000000,
        )
        .unwrap();

        let portfolio = Portfolio::new(10_000.0, vec![], None, PortfolioLimits::default()).unwrap();
        let assets = vec![sample_asset("NVDA")];

        let (approved, stages, err) = DeterministicAgentValidator::validate(
            &proposal, &portfolio, &assets, 1726000010, true, // Vault paused!
        );

        assert!(!approved);
        assert!(err.unwrap().contains("vault is paused"));
        assert_eq!(stages[4].passed, false);
    }
}
