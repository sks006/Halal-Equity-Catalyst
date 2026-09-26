//! Market data REST and real-time WebSocket endpoints.
//!
//! Exposes validated market data produced by the Pyth ingestion engine
//! without allowing frontend consumers to modify backend subscription state.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, trace, warn};

use crate::{
    error::ApiError,
    services::{MarketDataStore, MarketPriceUpdate, PriceFreshness},
    state::AppState,
};

/// Public market data payload returned by REST queries and streamed via WebSockets.
///
/// Contains comprehensive identity, pricing, and temporal validity metadata
/// without exposing internal synchronization primitives.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketDataResponse {
    /// Canonical asset identity (e.g. "backed:AAPLx")
    pub asset_id: String,
    /// Ticker symbol for display (e.g. "AAPL")
    pub symbol: String,
    /// Underlying Solana token mint address
    pub mint_address: String,
    /// Pyth feed identifier (normalized hex, without 0x prefix)
    pub pyth_feed_id: String,

    /// Authoritative scaled integer price in micro-USD ($1.00 = 1,000,000 micro-USD)
    pub price_scaled: u64,
    /// Raw integer price mantissa reported by Pyth oracle
    pub raw_price: i64,
    /// Pyth decimal exponent (e.g. -8)
    pub expo: i32,

    /// Scaled confidence interval in micro-USD
    pub conf_scaled: u64,
    /// Raw confidence interval reported by Pyth oracle
    pub conf_raw: u64,
    /// Confidence width in basis points relative to price (e.g. 5 = 0.05%)
    pub conf_bps: u16,

    /// Human-friendly display price in USD
    pub price_usd: f64,
    /// Display confidence interval in USD
    pub conf_usd: f64,

    /// Unix publication timestamp in seconds
    pub publish_time: i64,
    /// Unix store ingestion timestamp in seconds
    pub received_at: i64,
    /// Elapsed age in seconds since publication
    pub staleness_age_secs: i64,
    /// Whether the price is stale against the 30-second freshness window
    pub is_stale: bool,
    /// Categorical freshness status (Fresh, Stale, FutureSkew)
    pub freshness_status: PriceFreshness,

    /// Optional Solana/Pyth slot number for proof provenance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<u64>,
}

impl From<MarketPriceUpdate> for MarketDataResponse {
    fn from(update: MarketPriceUpdate) -> Self {
        let freshness_status = update.freshness_status(30);
        Self {
            asset_id: update.asset_id,
            symbol: update.symbol,
            mint_address: update.mint_address,
            pyth_feed_id: update.pyth_feed_id,
            price_scaled: update.price_scaled,
            raw_price: update.raw_price,
            expo: update.expo,
            conf_scaled: update.conf_scaled,
            conf_raw: update.conf_raw,
            conf_bps: update.conf_bps,
            price_usd: update.price_usd,
            conf_usd: update.conf_usd,
            publish_time: update.publish_time,
            received_at: update.received_at,
            staleness_age_secs: update.staleness_age_secs,
            is_stale: update.is_stale,
            freshness_status,
            slot: update.slot,
        }
    }
}

/// GET /market-data/:asset_id
///
/// Retrieves the latest validated market price and freshness metadata for an asset.
///
/// Accepts canonical `asset_id` (primary key), SPL token `mint_address`,
/// `pyth_feed_id`, or ticker `symbol`.
pub async fn get_market_data_handler(
    State(state): State<Arc<AppState>>,
    Path(asset_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let store = state.market_data_store.as_ref().ok_or_else(|| {
        ApiError::InternalServerError("Market data store is not initialized".to_string())
    })?;

    let update = store.get_latest(&asset_id).await.ok_or_else(|| {
        ApiError::NotFound(format!("Market data not found for asset: {}", asset_id))
    })?;

    let response = MarketDataResponse::from(update);
    Ok((StatusCode::OK, Json(response)))
}

/// GET /market-data/ws
///
/// Upgrades HTTP connection to a real-time WebSocket feed emitting `MarketPriceUpdate`
/// events as JSON payloads.
///
/// Invariants:
/// - Client connections are strictly consumers; incoming client messages never modify subscriptions.
/// - Slow or lagged clients do not block the central market data store.
/// - Client disconnects and transmission errors are handled gracefully without worker crashes.
pub async fn market_data_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    let store = match state.market_data_store.as_ref() {
        Some(s) => Arc::clone(s),
        None => {
            return ApiError::InternalServerError(
                "Market data store is not initialized".to_string(),
            )
            .into_response();
        }
    };

    ws.on_upgrade(move |socket| handle_market_data_socket(socket, store))
}

/// Manages the WebSocket lifecycle for a single market data client.
async fn handle_market_data_socket(socket: WebSocket, store: Arc<MarketDataStore>) {
    let mut rx = store.subscribe();
    let (mut sender, mut receiver) = socket.split();

    debug!("Market data WebSocket client connected");

    loop {
        tokio::select! {
            // Inbound messages from client (read-only consumer)
            client_msg = receiver.next() => {
                match client_msg {
                    Some(Ok(Message::Close(_))) | None => {
                        debug!("Market data WebSocket client closed connection");
                        break;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            debug!("Failed to send pong to WebSocket client; disconnecting");
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Heartbeat acknowledgement, ignore
                    }
                    Some(Ok(Message::Text(_))) | Some(Ok(Message::Binary(_))) => {
                        // INVARIANT: The frontend is a consumer, not an authority.
                        // Client input MUST NEVER modify Pyth subscriptions.
                        trace!("Ignoring unsolicited message from client on read-only market data stream");
                    }
                    Some(Err(e)) => {
                        debug!(error = %e, "Market data WebSocket client error; terminating connection");
                        break;
                    }
                }
            }

            // Outbound live market updates from MarketDataStore
            price_event = rx.recv() => {
                match price_event {
                    Ok(update) => {
                        let response = MarketDataResponse::from(update);
                        match serde_json::to_string(&response) {
                            Ok(json_str) => {
                                if sender.send(Message::Text(json_str)).await.is_err() {
                                    debug!("Failed to deliver market update to WebSocket client; disconnecting");
                                    break;
                                }
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to serialize market price update for WebSocket");
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        // INVARIANT: Slow clients must not block market-data ingestion.
                        // Broadcast channel dropped stale frames; log and catch up.
                        warn!(
                            skipped = skipped,
                            "Slow WebSocket client lagged behind broadcast stream; skipping stale updates"
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("Market data broadcast stream closed; terminating WebSocket session");
                        break;
                    }
                }
            }
        }
    }

    debug!("Market data WebSocket session terminated cleanly");
}
