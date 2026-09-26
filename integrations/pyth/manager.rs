//! Dynamic stream manager bridging subscription sets with Pyth SSE connections.
//!
//! Manages the lifecycle of the Pyth Hermes SSE stream:
//! - Subscribes to dynamic `SubscriptionSet` changes via `tokio::sync::watch`.
//! - Automatically drops and restarts the Pyth stream when subscriptions change.
//! - Idles without opening connections when the subscription set is empty.
//! - Reconnects with bounded exponential backoff upon Pyth disconnects or stream failures.
//! - Forwards parsed price events to a configured channel or callback sink.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{debug, error, info, warn};

use crate::stream::{PythStreamClient, PythStreamConfig};
use crate::subscription::{EnrichedPriceUpdate, SubscriptionSet};
use crate::types::PythPriceUpdateEvent;

/// Sink for receiving parsed or enriched Pyth price update events.
#[derive(Clone)]
pub enum PriceUpdateSink {
    /// Dispatches raw events to an asynchronous `mpsc::Sender`.
    Channel(mpsc::Sender<PythPriceUpdateEvent>),
    /// Dispatches enriched events preserving asset identity and Pyth data.
    EnrichedChannel(mpsc::Sender<EnrichedPriceUpdate>),
    /// Invokes a thread-safe callback closure for each raw event.
    Callback(Arc<dyn Fn(PythPriceUpdateEvent) + Send + Sync>),
    /// Invokes a thread-safe callback closure for each enriched update.
    EnrichedCallback(Arc<dyn Fn(EnrichedPriceUpdate) + Send + Sync>),
}

impl PriceUpdateSink {
    /// Forwards a price update event into the sink, enriching with canonical metadata if required.
    pub async fn send(&self, event: PythPriceUpdateEvent, sub_set: &SubscriptionSet) {
        match self {
            Self::Channel(tx) => {
                let _ = tx.send(event).await;
            }
            Self::EnrichedChannel(tx) => {
                for enriched in sub_set.enrich_event(&event) {
                    let _ = tx.send(enriched).await;
                }
            }
            Self::Callback(cb) => {
                cb(event);
            }
            Self::EnrichedCallback(cb) => {
                for enriched in sub_set.enrich_event(&event) {
                    cb(enriched);
                }
            }
        }
    }
}

/// Configuration parameters for `DynamicStreamManager`.
#[derive(Debug, Clone)]
pub struct StreamManagerConfig {
    /// Base URL for Pyth Hermes service (e.g. `https://hermes.pyth.network`).
    pub base_url: String,
    /// Optional authentication API key.
    pub api_key: Option<String>,
    /// Initial backoff delay after connection failure.
    pub initial_backoff: Duration,
    /// Maximum backoff ceiling.
    pub max_backoff: Duration,
    /// Multiplier applied to backoff after each consecutive failure.
    pub backoff_multiplier: f64,
    /// Maximum stream duration before clean rotation (handles documented 24-hour stream termination).
    pub max_stream_duration: Duration,
}

impl Default for StreamManagerConfig {
    fn default() -> Self {
        Self {
            base_url: "https://hermes.pyth.network".to_string(),
            api_key: None,
            initial_backoff: Duration::from_millis(250),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            max_stream_duration: Duration::from_secs(24 * 60 * 60), // Documented 24-hour termination
        }
    }
}

impl StreamManagerConfig {
    /// Creates a configuration with default backoff parameters for the given Hermes base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            ..Default::default()
        }
    }

    /// Attaches an authentication API key.
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Customizes exponential backoff parameters.
    pub fn with_backoff(mut self, initial: Duration, max: Duration, multiplier: f64) -> Self {
        self.initial_backoff = initial;
        self.max_backoff = max;
        self.backoff_multiplier = multiplier;
        self
    }

    /// Customizes maximum stream duration before proactive clean rotation (default: 24h).
    pub fn with_max_stream_duration(mut self, duration: Duration) -> Self {
        self.max_stream_duration = duration;
        self
    }
}

/// Dynamic manager coordinating runtime subscription sets and Pyth SSE streams.
pub struct DynamicStreamManager {
    config: StreamManagerConfig,
    subscription_rx: watch::Receiver<SubscriptionSet>,
    sink: PriceUpdateSink,
    http_client: reqwest::Client,
}

impl DynamicStreamManager {
    /// Creates a new `DynamicStreamManager` with the specified configuration, subscription receiver, and sink.
    pub fn new(
        config: StreamManagerConfig,
        subscription_rx: watch::Receiver<SubscriptionSet>,
        sink: PriceUpdateSink,
    ) -> Self {
        Self {
            config,
            subscription_rx,
            sink,
            http_client: reqwest::Client::builder().build().unwrap_or_default(),
        }
    }

    /// Convenience constructor routing raw updates to an `mpsc::Sender`.
    pub fn with_channel(
        config: StreamManagerConfig,
        subscription_rx: watch::Receiver<SubscriptionSet>,
        tx: mpsc::Sender<PythPriceUpdateEvent>,
    ) -> Self {
        Self::new(config, subscription_rx, PriceUpdateSink::Channel(tx))
    }

    /// Convenience constructor routing enriched updates preserving asset identity and Pyth data.
    pub fn with_enriched_channel(
        config: StreamManagerConfig,
        subscription_rx: watch::Receiver<SubscriptionSet>,
        tx: mpsc::Sender<EnrichedPriceUpdate>,
    ) -> Self {
        Self::new(
            config,
            subscription_rx,
            PriceUpdateSink::EnrichedChannel(tx),
        )
    }

    /// Convenience constructor routing raw updates to a callback closure.
    pub fn with_callback<F>(
        config: StreamManagerConfig,
        subscription_rx: watch::Receiver<SubscriptionSet>,
        cb: F,
    ) -> Self
    where
        F: Fn(PythPriceUpdateEvent) + Send + Sync + 'static,
    {
        Self::new(
            config,
            subscription_rx,
            PriceUpdateSink::Callback(Arc::new(cb)),
        )
    }

    /// Convenience constructor routing enriched updates to a callback closure.
    pub fn with_enriched_callback<F>(
        config: StreamManagerConfig,
        subscription_rx: watch::Receiver<SubscriptionSet>,
        cb: F,
    ) -> Self
    where
        F: Fn(EnrichedPriceUpdate) + Send + Sync + 'static,
    {
        Self::new(
            config,
            subscription_rx,
            PriceUpdateSink::EnrichedCallback(Arc::new(cb)),
        )
    }

    /// Reuses a custom `reqwest::Client`.
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = client;
        self
    }

    /// Spawns the stream manager onto the Tokio runtime.
    pub fn spawn(self, shutdown: broadcast::Receiver<()>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.run(shutdown).await;
        })
    }

    /// Runs the dynamic supervision loop.
    pub async fn run(mut self, mut shutdown: broadcast::Receiver<()>) {
        info!(
            base_url = %self.config.base_url,
            "Starting Pyth DynamicStreamManager supervision loop"
        );

        let mut current_set = self.subscription_rx.borrow().clone();
        let mut current_backoff = self.config.initial_backoff;

        loop {
            // Task 7: Never reconnect with a stale subscription list.
            let latest_set = self.subscription_rx.borrow().clone();
            if latest_set != current_set {
                debug!(
                    prev = current_set.len(),
                    latest = latest_set.len(),
                    "DynamicStreamManager refreshed to latest SubscriptionSet before connect"
                );
                current_set = latest_set;
                current_backoff = self.config.initial_backoff;
            }

            // Task 6: If the subscription is empty, do not open a Pyth stream. Wait for next update.
            if current_set.is_empty() {
                debug!("DynamicStreamManager: SubscriptionSet is empty; idling until subscriptions are added");
                tokio::select! {
                    _ = shutdown.recv() => {
                        info!("DynamicStreamManager received shutdown signal while idle");
                        return;
                    }
                    res = self.subscription_rx.changed() => {
                        if res.is_err() {
                            info!("Subscription watch channel closed; exiting DynamicStreamManager");
                            return;
                        }
                        let new_set = self.subscription_rx.borrow().clone();
                        if new_set != current_set {
                            info!(count = new_set.len(), "DynamicStreamManager observed new non-empty subscription set");
                            current_set = new_set;
                            current_backoff = self.config.initial_backoff;
                        }
                        continue;
                    }
                }
            }

            // Task 8 & 10: Deduplicate and sort feed IDs strictly from approved subscription set
            let mut feed_ids = current_set.feed_ids();
            feed_ids.sort();
            feed_ids.dedup();

            info!(
                feed_count = feed_ids.len(),
                feed_ids = ?feed_ids,
                "Connecting Pyth SSE stream with dynamic subscription set"
            );

            let stream_config = {
                let mut sc = PythStreamConfig::new(&self.config.base_url, feed_ids);
                if let Some(ref key) = self.config.api_key {
                    sc = sc.with_api_key(key);
                }
                sc
            };

            let stream_client =
                PythStreamClient::new_with_http_client(stream_config, self.http_client.clone());
            let connect_res = stream_client.connect().await;

            // Task 7 check: Did subscription change during connect handshake?
            let post_connect_set = self.subscription_rx.borrow().clone();
            if post_connect_set != current_set {
                info!("Subscription changed during connect handshake; discarding stream to reconnect with fresh set");
                current_set = post_connect_set;
                current_backoff = self.config.initial_backoff;
                continue;
            }

            let mut active_stream = match connect_res {
                Ok(stream) => {
                    info!("Connected to Pyth SSE stream successfully");
                    current_backoff = self.config.initial_backoff;
                    stream
                }
                Err(err) => {
                    error!(
                        error = %err,
                        backoff_ms = current_backoff.as_millis(),
                        "Failed to connect to Pyth SSE stream; backing off"
                    );

                    // Task 4: Exponential backoff delay upon failure
                    tokio::select! {
                        _ = shutdown.recv() => {
                            info!("DynamicStreamManager received shutdown signal during backoff");
                            return;
                        }
                        res = self.subscription_rx.changed() => {
                            if res.is_err() {
                                return;
                            }
                            let new_set = self.subscription_rx.borrow().clone();
                            if new_set != current_set {
                                info!("Subscription changed during backoff; canceling backoff and reconnecting immediately");
                                current_set = new_set;
                                current_backoff = self.config.initial_backoff;
                            }
                            continue;
                        }
                        _ = tokio::time::sleep(current_backoff) => {
                            let next_backoff = (current_backoff.as_secs_f64() * self.config.backoff_multiplier)
                                .min(self.config.max_backoff.as_secs_f64());
                            current_backoff = Duration::from_secs_f64(next_backoff);
                            continue;
                        }
                    }
                }
            };

            // Stream lifetime tracker for Task 5: 24-hour stream termination handling
            let max_duration = self.config.max_stream_duration;
            let stream_timeout = tokio::time::sleep(max_duration);
            tokio::pin!(stream_timeout);

            // Task 3: Stream event and subscription change loop
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        info!("DynamicStreamManager received shutdown signal; dropping active stream");
                        return;
                    }

                    // Task 5: Handle documented 24-hour stream termination (clean proactive rotation)
                    _ = &mut stream_timeout => {
                        info!("Pyth SSE stream reached maximum connection lifetime (24 hours); rotating stream cleanly");
                        current_backoff = self.config.initial_backoff;
                        break;
                    }

                    // Task 3: When subscription changes, drop current stream, construct new stream, reconnect
                    res = self.subscription_rx.changed() => {
                        if res.is_err() {
                            info!("Subscription watch channel closed; exiting DynamicStreamManager");
                            return;
                        }
                        let new_set = self.subscription_rx.borrow().clone();
                        if new_set != current_set {
                            info!(
                                prev_count = current_set.len(),
                                new_count = new_set.len(),
                                prev_feeds = ?current_set.feed_ids(),
                                new_feeds = ?new_set.feed_ids(),
                                "SubscriptionSet changed; dropping active Pyth stream to reconnect"
                            );
                            current_set = new_set;
                            current_backoff = self.config.initial_backoff;
                            // Dropping `active_stream` by breaking to outer loop
                            break;
                        }
                    }

                    // Task 3A & 4: Pyth stream events/termination
                    maybe_update = active_stream.next_update() => {
                        match maybe_update {
                            Some(Ok(event)) => {
                                // Task 9: Dispatches event, preserving feed ID, asset ID, mint, price, conf, publish time
                                self.sink.send(event, &current_set).await;
                            }
                            Some(Err(err)) => {
                                error!(error = %err, "Pyth SSE stream error encountered; reconnecting with backoff");
                                // Break to outer loop to drop stream and reconnect
                                break;
                            }
                            None => {
                                // Task 5: Handle server-closed stream (clean EOF / 24-hour disconnect)
                                warn!("Pyth SSE stream terminated by server (clean EOF / 24-hour stream termination); reconnecting");
                                current_backoff = self.config.initial_backoff;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}
