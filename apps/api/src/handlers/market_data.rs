//! Market data REST and real-time WebSocket endpoints.
//!
//! Conforms to Single Responsibility Principle (SRP):
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketDataResponse {
    pub asset_id: String,
    pub symbol: String,
    pub mint_address: String,
    pub pyth_feed_id: String,
    pub price_scaled: u64,
    pub raw_price: i64,
    pub expo: i32,
    pub conf_scaled: u64,
    pub conf_raw: u64,
    pub conf_bps: u16,
    pub price_usd: f64,
    pub conf_usd: f64,
    pub publish_time: i64,
    pub received_at: i64,
    pub staleness_age_secs: i64,
    pub is_stale: bool,
    pub freshness_status: PriceFreshness,
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

async fn handle_market_data_socket(socket: WebSocket, store: Arc<MarketDataStore>) {
    let mut rx = store.subscribe();
    let (mut sender, mut receiver) = socket.split();

    debug!("Market data WebSocket client connected");

    loop {
        tokio::select! {
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
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Text(_))) | Some(Ok(Message::Binary(_))) => {
                        trace!("Ignoring unsolicited message from client on read-only market data stream");
                    }
                    Some(Err(e)) => {
                        debug!(error = %e, "Market data WebSocket client error; terminating connection");
                        break;
                    }
                }
            }

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
