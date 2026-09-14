//! Worker consuming queued events, evaluating policy conditions, performing risk checks,
//! and logging execution decisions (without executing live trades on-chain).

use chrono::Utc;
use redis::AsyncCommands;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info};

use crate::{
    engines::decision_engine::{DecisionEngine, ExecutionRequest},
    error::ApiError,
    models::{EventModel, ExecutionModel},
    repositories::{
        event_repository::EventRepository,
        execution_repository::ExecutionRepository,
        policy_repository::PolicyRepository,
        portfolio_repository::PortfolioRepository,
        vault_repository::VaultRepository,
    },
    workers::event_listener::DEFAULT_EVENTS_QUEUE,
};

#[derive(Clone)]
pub struct PolicyWorker {
    decision_engine: DecisionEngine,
    vault_repo: VaultRepository,
    policy_repo: PolicyRepository,
    portfolio_repo: PortfolioRepository,
    event_repo: EventRepository,
    execution_repo: ExecutionRepository,
    redis_client: Option<redis::Client>,
    queue_key: String,
}

impl PolicyWorker {
    pub fn new(
        decision_engine: DecisionEngine,
        vault_repo: VaultRepository,
        policy_repo: PolicyRepository,
        portfolio_repo: PortfolioRepository,
        event_repo: EventRepository,
        execution_repo: ExecutionRepository,
        redis_client: Option<redis::Client>,
    ) -> Self {
        Self {
            decision_engine,
            vault_repo,
            policy_repo,
            portfolio_repo,
            event_repo,
            execution_repo,
            redis_client,
            queue_key: DEFAULT_EVENTS_QUEUE.to_string(),
        }
    }

    pub fn with_queue_key(mut self, key: &str) -> Self {
        self.queue_key = key.to_string();
        self
    }

    pub fn queue_key(&self) -> &str {
        &self.queue_key
    }

    /// Evaluates a single event through the full policy and risk pipeline:
    /// `event -> load vault policy -> evaluate conditions -> risk check -> decision`
    ///
    /// In accordance with Phase 9 instructions:
    /// - Never automatically executes a trade.
    /// - Logs the decision.
    /// - Persists decision audit log to the `executions` table with status 'LOGGED'.
    /// - Updates event status to 'PROCESSED'.
    pub async fn process_single_event(
        &self,
        event: &EventModel,
    ) -> Result<ExecutionRequest, ApiError> {
        let vault_addr = event.vault_address.as_deref().ok_or_else(|| {
            ApiError::BadRequest(format!("Event {} does not have a linked vault_address", event.event_id))
        })?;

        // 1. Load Vault
        let vault = self
            .vault_repo
            .find_by_address(vault_addr)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Vault not found for address: {}", vault_addr)))?;

        // 2. Load Policy
        let policy = self
            .policy_repo
            .find_by_vault(vault_addr)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Policy not found for vault: {}", vault_addr)))?;

        // 3. Load Portfolio positions
        let positions = self.portfolio_repo.list_by_vault(vault_addr).await?;
        let total_value_usd: u64 = positions.iter().map(|p| p.current_value_usd.max(0.0) as u64).sum();

        // 4. Evaluate: Policy Engine -> Risk Engine -> Decision Engine
        let decision = self.decision_engine.process_event(
            event,
            &vault,
            &policy,
            &positions,
            total_value_usd,
            0,
        )?;

        // 5. DRY-RUN LOGGING (Crucial: First log the decision, do not execute)
        info!(
            event_id = %event.event_id,
            decision_id = %decision.decision_id,
            action = %decision.action,
            trades_count = decision.trades.len(),
            approved = decision.approved,
            rationale = %decision.rationale,
            "Policy Worker evaluated decision — TRADE EXECUTION SUPPRESSED (DRY-RUN MODE)"
        );

        // 6. Record decision in `executions` audit table with status 'LOGGED'
        let (input_mint, output_mint, amount_in, amount_out) = if let Some(first_trade) = decision.trades.first() {
            (
                vault.deposit_mint.clone(),
                vault.deposit_mint.clone(),
                first_trade.usd_value,
                first_trade.usd_value,
            )
        } else {
            (
                vault.deposit_mint.clone(),
                vault.deposit_mint.clone(),
                0,
                0,
            )
        };

        let execution_record = ExecutionModel {
            execution_id: decision.decision_id,
            vault_address: vault.vault_address.clone(),
            event_id: Some(event.event_id),
            action: decision.action.clone(),
            input_mint,
            output_mint,
            amount_in,
            amount_out_expected: amount_out,
            amount_out_actual: None,
            slippage_bps: 100, // Default 1%
            tx_signature: None, // Suppressed in dry-run mode
            status: "LOGGED".to_string(), // Explicitly recorded as LOGGED, not broadcast
            error_message: if decision.approved {
                None
            } else {
                Some(decision.rationale.clone())
            },
            executed_at: Utc::now(),
            confirmed_at: None,
        };

        let _ = self.execution_repo.create(&execution_record).await;

        // 7. Mark event status as PROCESSED
        self.event_repo
            .update_status(event.event_id, "PROCESSED", Some(Utc::now()))
            .await?;

        Ok(decision)
    }

    /// Fetches and processes the next pending event from Redis or Postgres fallback.
    pub async fn poll_and_process_next(&self) -> Result<Option<ExecutionRequest>, ApiError> {
        // 1. Try to pop from Redis FIFO queue
        if let Some(ref client) = self.redis_client {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                let maybe_json: Result<Option<String>, _> = conn.lpop(&self.queue_key, None).await;
                if let Ok(Some(json_str)) = maybe_json {
                    if let Ok(event) = serde_json::from_str::<EventModel>(&json_str) {
                        debug!(event_id = %event.event_id, "Popped event from Redis queue");
                        let decision = self.process_single_event(&event).await?;
                        return Ok(Some(decision));
                    }
                }
            }
        }

        // 2. Fallback: Query pending events from PostgreSQL
        let pending = self.event_repo.find_pending().await?;
        if let Some(event) = pending.first() {
            debug!(event_id = %event.event_id, "Fetched pending event from PostgreSQL queue");
            let decision = self.process_single_event(event).await?;
            return Ok(Some(decision));
        }

        Ok(None)
    }

    /// Long-lived worker loop polling and processing events.
    pub async fn run(&self, mut shutdown: broadcast::Receiver<()>) -> Result<(), ApiError> {
        info!("Starting PolicyWorker processing loop");
        let idle_sleep = Duration::from_millis(500);

        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    info!("PolicyWorker received shutdown signal");
                    break;
                }
                res = self.poll_and_process_next() => {
                    match res {
                        Ok(Some(decision)) => {
                            debug!(decision_id = %decision.decision_id, "Successfully processed queued event");
                        }
                        Ok(None) => {
                            tokio::time::sleep(idle_sleep).await;
                        }
                        Err(err) => {
                            error!(error = %err, "Error in PolicyWorker loop");
                            tokio::time::sleep(idle_sleep).await;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
