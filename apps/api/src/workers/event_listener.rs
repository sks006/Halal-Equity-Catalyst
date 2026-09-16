//! Worker listening to Solana WebSocket event streams, parsing Anchor program logs,
//! persisting events to PostgreSQL, and publishing to Redis queues for policy evaluation.

use chrono::Utc;
use equity_catalyst_solana::{accounts::*, websocket::LogsNotification};
use redis::AsyncCommands;
use serde_json::json;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    error::ApiError, models::EventModel, repositories::event_repository::EventRepository,
    services::SolanaService,
};

pub const DEFAULT_EVENTS_QUEUE: &str = "equity:events:queue";
pub const DEFAULT_EVENTS_CHANNEL: &str = "equity:events:channel";

#[derive(Clone)]
pub struct EventListener {
    solana_service: SolanaService,
    event_repo: EventRepository,
    redis_client: Option<redis::Client>,
    queue_key: String,
    channel_key: String,
}

impl EventListener {
    pub fn new(
        solana_service: SolanaService,
        event_repo: EventRepository,
        redis_client: Option<redis::Client>,
    ) -> Self {
        Self {
            solana_service,
            event_repo,
            redis_client,
            queue_key: DEFAULT_EVENTS_QUEUE.to_string(),
            channel_key: DEFAULT_EVENTS_CHANNEL.to_string(),
        }
    }

    pub fn with_queue_key(mut self, key: &str) -> Self {
        self.queue_key = key.to_string();
        self
    }

    pub fn with_channel_key(mut self, key: &str) -> Self {
        self.channel_key = key.to_string();
        self
    }

    pub fn queue_key(&self) -> &str {
        &self.queue_key
    }

    pub fn channel_key(&self) -> &str {
        &self.channel_key
    }

    /// Ingests and processes a single `LogsNotification` from Solana:
    /// 1. Parses Anchor event payload from logs.
    /// 2. Persists EventModel into PostgreSQL `events` table (status: PENDING).
    /// 3. Pushes event payload to Redis queue (`RPUSH`) and pub/sub channel.
    pub async fn process_notification(
        &self,
        notification: &LogsNotification,
    ) -> Result<Option<EventModel>, ApiError> {
        let mut detected_event: Option<(String, Option<String>, serde_json::Value)> = None;

        // 1. First attempt: Parse binary Anchor event data (`Program data: <base64>`)
        for log in &notification.logs {
            if let Some(parsed) = parse_program_data_log(log) {
                let event_type = parsed.event_type().to_string();
                let vault_addr = Some(parsed.vault_address());
                let payload = serde_json::to_value(&parsed).unwrap_or(json!({}));
                detected_event = Some((event_type, vault_addr, payload));
                break;
            }
        }

        // 2. Fallback attempt: Parse instruction log lines (e.g., `Program log: Instruction: Deposit`)
        if detected_event.is_none() {
            for log in &notification.logs {
                if let Some(rest) = log.strip_prefix("Program log: Instruction: ") {
                    let ix_name = rest.trim();
                    let (event_type, action) = match ix_name {
                        "Deposit" => ("DEPOSIT", "deposit"),
                        "Withdraw" => ("WITHDRAW", "withdraw"),
                        "UpdatePolicy" => ("POLICY_UPDATED", "update_policy"),
                        "EmergencyExit" => ("VAULT_PAUSE_TOGGLED", "emergency_exit"),
                        "InitializeVault" => ("VAULT_INITIALIZED", "initialize_vault"),
                        _ => continue,
                    };

                    let payload = json!({
                        "instruction": action,
                        "signature": notification.signature,
                    });

                    detected_event = Some((event_type.to_string(), None, payload));
                    break;
                }
            }
        }

        // If no relevant program event was detected in logs, skip
        let (event_type, vault_address, mut payload) = match detected_event {
            Some(data) => data,
            None => return Ok(None),
        };

        if let Some(obj) = payload.as_object_mut() {
            obj.insert("tx_signature".to_string(), json!(notification.signature));
        }

        // 3. Persist EventModel into PostgreSQL
        let new_event = EventModel {
            event_id: Uuid::new_v4(),
            vault_address,
            event_type,
            source: "solana_websocket".to_string(),
            sentiment_score: None,
            payload,
            status: "PENDING".to_string(),
            detected_at: Utc::now(),
            processed_at: None,
        };

        let persisted = self.event_repo.create(&new_event).await?;
        info!(
            event_id = %persisted.event_id,
            event_type = %persisted.event_type,
            vault = ?persisted.vault_address,
            "Persisted on-chain event to PostgreSQL"
        );

        // 4. Push to Redis event queue and pub/sub
        if let Some(ref client) = self.redis_client {
            match client.get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    let serialized = serde_json::to_string(&persisted)
                        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

                    // Push to durable FIFO queue
                    let _: Result<(), _> = conn.rpush(&self.queue_key, &serialized).await;

                    // Publish to real-time pubsub channel
                    let _: Result<(), _> = conn.publish(&self.channel_key, &serialized).await;

                    debug!(
                        event_id = %persisted.event_id,
                        queue = %self.queue_key,
                        "Dispatched event to Redis queue"
                    );
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        "Failed to connect to Redis for event queuing; relying on PostgreSQL pending queue"
                    );
                }
            }
        }

        Ok(Some(persisted))
    }

    /// Runs the long-lived event listening loop.
    pub async fn run(&self, mut shutdown: broadcast::Receiver<()>) -> Result<(), ApiError> {
        info!("Starting Solana EventListener worker");
        let mut rx = self.solana_service.subscribe_events().await?;

        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    info!("EventListener received shutdown signal");
                    break;
                }
                maybe_notification = rx.recv() => {
                    match maybe_notification {
                        Some(notification) => {
                            if let Err(err) = self.process_notification(&notification).await {
                                error!(error = %err, "Error processing logs notification");
                            }
                        }
                        None => {
                            warn!("Solana WebSocket channel closed, exiting EventListener");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
