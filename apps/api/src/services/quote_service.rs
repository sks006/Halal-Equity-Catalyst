//! Quote execution domain service integrating Jupiter v6 quotes with the Risk Engine.

use chrono::{DateTime, Utc};
use equity_catalyst_jupiter::{
    parse_price_impact_bps, JupiterClient, QuoteRequest, QuoteResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    engines::risk_engine::{validate_position_exposure, RiskEngine},
    error::ApiError,
    models::ExecutionModel,
    repositories::{ExecutionRepository, PolicyRepository, PortfolioRepository, VaultRepository},
};

/// Request parameters for evaluating a trade quote without on-chain execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteExecutionRequest {
    pub vault_address: String,
    pub input_mint: String,
    pub output_mint: String,
    pub amount_in: u64,
    #[serde(default)]
    pub slippage_bps: Option<u16>,
    #[serde(default)]
    pub target_symbol: Option<String>,
}

/// Structured outcome of a quote-only execution pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuoteExecutionVerdict {
    pub execution_id: Uuid,
    pub vault_address: String,
    pub input_mint: String,
    pub output_mint: String,
    pub amount_in: u64,
    pub expected_amount_out: u64,
    pub min_amount_out: u64,
    pub price_impact_bps: u16,
    pub price_impact_pct: String,
    pub approved: bool,
    pub rejection_reason: Option<String>,
    pub evaluated_exposure_bps: Option<u16>,
    pub is_dry_run: bool,
    pub evaluated_at: DateTime<Utc>,
}

/// Service implementing Step 33 quote-only execution.
/// Flow: token A -> Jupiter quote -> expected output -> price impact -> risk engine
#[derive(Clone)]
pub struct QuoteExecutionService {
    jupiter_client: Arc<JupiterClient>,
    risk_engine: Arc<RiskEngine>,
    vault_repo: Option<VaultRepository>,
    policy_repo: Option<PolicyRepository>,
    portfolio_repo: Option<PortfolioRepository>,
    execution_repo: Option<ExecutionRepository>,
}

impl QuoteExecutionService {
    pub fn new(
        jupiter_client: Arc<JupiterClient>,
        risk_engine: Arc<RiskEngine>,
        vault_repo: Option<VaultRepository>,
        policy_repo: Option<PolicyRepository>,
        portfolio_repo: Option<PortfolioRepository>,
        execution_repo: Option<ExecutionRepository>,
    ) -> Self {
        Self {
            jupiter_client,
            risk_engine,
            vault_repo,
            policy_repo,
            portfolio_repo,
            execution_repo,
        }
    }

    pub fn risk_engine(&self) -> &RiskEngine {
        &self.risk_engine
    }

    pub fn jupiter_client(&self) -> &JupiterClient {
        &self.jupiter_client
    }

    /// Evaluates a trade quote through the complete multi-factor risk defense pipeline.
    /// In accordance with Step 33: "Do not submit real swaps yet."
    pub async fn evaluate_quote(
        &self,
        request: &QuoteExecutionRequest,
    ) -> Result<QuoteExecutionVerdict, ApiError> {
        let execution_id = Uuid::new_v4();
        let slippage = request.slippage_bps.unwrap_or(50); // Default 0.50%

        // 1. token A -> Jupiter quote
        let quote_req = QuoteRequest::new(&request.input_mint, &request.output_mint, request.amount_in)
            .with_slippage_bps(slippage);

        let quote: QuoteResponse = self
            .jupiter_client
            .get_quote(&quote_req)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Jupiter quote failed: {}", e)))?;

        // 2. Expected output & other amount threshold
        let expected_amount_out: u64 = quote
            .out_amount
            .parse()
            .map_err(|e| ApiError::InternalServerError(format!("Failed to parse out_amount: {}", e)))?;

        let min_amount_out: u64 = quote
            .other_amount_threshold
            .parse()
            .unwrap_or(expected_amount_out);

        // 3. Price impact
        let price_impact_bps = parse_price_impact_bps(&quote.price_impact_pct)
            .map_err(|e| ApiError::InternalServerError(format!("Invalid price impact: {}", e)))?;

        // 4. Risk engine evaluation
        let mut approved = true;
        let mut rejection_reason = None;
        let mut evaluated_exposure_bps = None;

        // Verify vault and policy if repositories are available
        if let (Some(ref v_repo), Some(ref p_repo)) = (&self.vault_repo, &self.policy_repo) {
            let vault = v_repo.find_by_address(&request.vault_address).await?;
            let policy = p_repo.find_by_vault(&request.vault_address).await?;

            if let (Some(vault), Some(policy)) = (vault, policy) {
                // Check if vault is paused
                if vault.is_paused {
                    approved = false;
                    rejection_reason = Some("Vault operations are currently paused".to_string());
                } else if !policy.is_active {
                    approved = false;
                    rejection_reason = Some("Vault risk policy is inactive".to_string());
                } else {
                    // Check price impact against allowed slippage / threshold
                    let max_allowed_impact = policy.rebalance_threshold_bps.min(1_000) as u16; // e.g. up to 10%
                    if price_impact_bps > max_allowed_impact {
                        approved = false;
                        rejection_reason = Some(format!(
                            "Price impact {} bps breaches maximum allowed limit {} bps",
                            price_impact_bps, max_allowed_impact
                        ));
                    }

                    // Check portfolio exposure if portfolio repository is available
                    if approved {
                        if let Some(ref port_repo) = self.portfolio_repo {
                            let positions = port_repo.list_by_vault(&request.vault_address).await?;
                            let total_portfolio_usd: u64 = positions
                                .iter()
                                .map(|p| p.current_value_usd.max(0.0) as u64)
                                .sum();

                            // Evaluate position exposure limit
                            let target_sym = request
                                .target_symbol
                                .clone()
                                .unwrap_or_else(|| "TARGET".to_string());

                            let current_pos_val = positions
                                .iter()
                                .find(|p| p.asset_symbol == target_sym || p.asset_mint == request.output_mint)
                                .map(|p| p.current_value_usd.max(0.0) as u64)
                                .unwrap_or(0);

                            // Estimate new trade value (amount_in micro-USD or proportional)
                            let trade_val = (request.amount_in as f64).min(total_portfolio_usd as f64) as u64;
                            let post_val = current_pos_val + trade_val;

                            if total_portfolio_usd > 0 {
                                let exposure_bps = ((post_val as f64 / total_portfolio_usd as f64) * 10_000.0).round() as u16;
                                evaluated_exposure_bps = Some(exposure_bps);

                                if let Err(reason) = validate_position_exposure(
                                    post_val,
                                    total_portfolio_usd,
                                    policy.max_position_bps as u16,
                                ) {
                                    approved = false;
                                    rejection_reason = Some(reason);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 5. Record execution audit entry as "QUOTE_ONLY" (suppressing live swap)
        if let Some(ref exec_repo) = self.execution_repo {
            let exec_model = ExecutionModel {
                execution_id,
                vault_address: request.vault_address.clone(),
                event_id: None,
                action: "QUOTE_EVALUATION".to_string(),
                input_mint: request.input_mint.clone(),
                output_mint: request.output_mint.clone(),
                amount_in: request.amount_in,
                amount_out_expected: expected_amount_out,
                amount_out_actual: None,
                slippage_bps: slippage as i32,
                tx_signature: None, // NO LIVE SWAP SUBMITTED
                status: if approved { "QUOTE_ONLY".to_string() } else { "REJECTED".to_string() },
                error_message: rejection_reason.clone(),
                executed_at: Utc::now(),
                confirmed_at: None,
            };

            let _ = exec_repo.create(&exec_model).await;
        }

        if approved {
            info!(
                execution_id = %execution_id,
                in_amount = request.amount_in,
                expected_out = expected_amount_out,
                impact_bps = price_impact_bps,
                "Quote approved by Risk Engine — LIVE SWAP SUPPRESSED (QUOTE-ONLY MODE)"
            );
        } else {
            warn!(
                execution_id = %execution_id,
                reason = ?rejection_reason,
                "Quote rejected by Risk Engine"
            );
        }

        Ok(QuoteExecutionVerdict {
            execution_id,
            vault_address: request.vault_address.clone(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            amount_in: request.amount_in,
            expected_amount_out,
            min_amount_out,
            price_impact_bps,
            price_impact_pct: quote.price_impact_pct,
            approved,
            rejection_reason,
            evaluated_exposure_bps,
            is_dry_run: true,
            evaluated_at: Utc::now(),
        })
    }
}
