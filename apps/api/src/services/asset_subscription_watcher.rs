//! Dynamic Pyth subscription watcher abstraction.
//!
//! Periodically inspects `AssetMarketDataRepository` for active approved assets,
//! builds a normalized `SubscriptionSet`, and notifies listeners via `tokio::sync::watch`
//! strictly when changes occur.
//!
//! Architectural Invariants:
//! 1. The watcher does NOT connect to Pyth.
//! 2. The watcher does NOT own the Pyth HTTP client.
//! 3. Polling interval is fully configurable per environment.
//! 4. Equality is deterministic; database row order variations do not cause false updates.

use equity_catalyst_pyth::SubscriptionSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch};
use tracing::{debug, error, info};

use crate::{
    error::ApiError, models::asset_market_data::subscription_set_from_mappings,
    repositories::AssetMarketDataRepository,
};

/// Configuration for `AssetSubscriptionWatcher`.
#[derive(Debug, Clone)]
pub struct SubscriptionWatcherConfig {
    /// Interval between repository polls.
    pub poll_interval: Duration,
}

impl Default for SubscriptionWatcherConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(5),
        }
    }
}

impl SubscriptionWatcherConfig {
    /// Creates a configuration with the specified polling interval.
    pub fn new(poll_interval: Duration) -> Self {
        Self { poll_interval }
    }
}

/// Dynamic watcher that monitors `AssetMarketDataRepository` and publishes
/// updated `SubscriptionSet` instances to listeners without application recompilation.
#[derive(Clone)]
pub struct AssetSubscriptionWatcher {
    repository: AssetMarketDataRepository,
    config: SubscriptionWatcherConfig,
    sender: Arc<watch::Sender<SubscriptionSet>>,
    receiver: watch::Receiver<SubscriptionSet>,
}

impl AssetSubscriptionWatcher {
    /// Creates a new watcher initialized with an empty `SubscriptionSet`.
    pub fn new(repository: AssetMarketDataRepository, config: SubscriptionWatcherConfig) -> Self {
        let initial_set = SubscriptionSet::empty();
        let (sender, receiver) = watch::channel(initial_set);
        Self {
            repository,
            config,
            sender: Arc::new(sender),
            receiver,
        }
    }

    /// Creates a new watcher, immediately performing an initial repository poll
    /// to seed the current `SubscriptionSet`.
    pub async fn new_initialized(
        repository: AssetMarketDataRepository,
        config: SubscriptionWatcherConfig,
    ) -> Result<Self, ApiError> {
        let active_mappings = repository.get_active_mappings().await?;
        let initial_set = subscription_set_from_mappings(&active_mappings);
        let (sender, receiver) = watch::channel(initial_set);
        Ok(Self {
            repository,
            config,
            sender: Arc::new(sender),
            receiver,
        })
    }

    /// Obtains a new `Receiver` for subscription set updates.
    ///
    /// Receivers can inspect the current set via `.borrow()` and await changes
    /// via `.changed().await`.
    pub fn subscribe(&self) -> watch::Receiver<SubscriptionSet> {
        self.receiver.clone()
    }

    /// Returns a clone of the current active `SubscriptionSet`.
    pub fn current_subscription_set(&self) -> SubscriptionSet {
        self.sender.borrow().clone()
    }

    /// Returns the configured polling interval.
    pub fn poll_interval(&self) -> Duration {
        self.config.poll_interval
    }

    /// Polls `AssetMarketDataRepository` once for active approved assets.
    ///
    /// If the newly retrieved set differs deterministically from the current set,
    /// listeners are notified and this method returns `Ok(true)`.
    /// Otherwise, no notification is sent and this method returns `Ok(false)`.
    pub async fn poll_once(&self) -> Result<bool, ApiError> {
        let active_mappings = self.repository.get_active_mappings().await?;
        let new_set = subscription_set_from_mappings(&active_mappings);

        let mut changed = false;
        self.sender.send_if_modified(|current| {
            if current != &new_set {
                debug!(
                    prev_count = current.len(),
                    new_count = new_set.len(),
                    "Detected change in active Pyth market data subscriptions"
                );
                *current = new_set;
                changed = true;
                true
            } else {
                false
            }
        });

        if changed {
            info!(
                active_subscriptions = self.sender.borrow().len(),
                feed_ids = ?self.sender.borrow().feed_ids(),
                "Published updated Pyth SubscriptionSet to listeners"
            );
        }

        Ok(changed)
    }

    /// Starts the background periodic polling loop.
    ///
    /// The returned `JoinHandle` can be used to monitor the task, and will exit
    /// gracefully when a signal is received on the `shutdown` broadcast channel.
    pub fn start(
        self: Arc<Self>,
        mut shutdown: broadcast::Receiver<()>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!(
                interval_ms = self.config.poll_interval.as_millis(),
                "Starting AssetSubscriptionWatcher background loop"
            );

            let mut interval = tokio::time::interval(self.config.poll_interval);
            // The first tick completes immediately; avoid duplicate poll if already initialized
            interval.tick().await;

            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        info!("AssetSubscriptionWatcher received shutdown signal, stopping watcher loop");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Err(err) = self.poll_once().await {
                            error!(error = %err, "AssetSubscriptionWatcher poll cycle failed");
                        }
                    }
                }
            }
        })
    }
}
