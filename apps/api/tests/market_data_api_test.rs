//! API Integration tests for Phase 8: Market Data REST and WebSocket API.
//!
//! Validates:
//! - REST endpoint: `GET /market-data/:asset_id` returns validated price and freshness.
//! - Secondary identifier resolution (mint, feed, symbol).
//! - Non-existent asset returns 404 Not Found.
//! - WebSocket endpoint: `GET /market-data/ws` streams `MarketPriceUpdate` events to clients.
//! - Consumer invariance: Frontend input does NOT modify Pyth subscriptions or backend state.
//! - Clean disconnection handling: Disconnected clients do not crash worker/server.
//! - Non-blocking delivery: Slow clients do not block market data ingestion or other clients.
//! - Full acceptance criteria: Frontend clients observe live validated market prices without directly accessing Pyth.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use equity_catalyst_api::{
    config::Config,
    create_db_pool, create_router,
    routes::MarketDataResponse,
    services::{MarketDataStore, MarketPriceUpdate, PriceFreshness},
    state::AppState,
};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tower::ServiceExt;

fn sample_update(
    asset_id: &str,
    symbol: &str,
    mint: &str,
    feed_id: &str,
    raw_price: i64,
    publish_time: i64,
) -> MarketPriceUpdate {
    MarketPriceUpdate::from_raw(
        asset_id,
        symbol,
        mint,
        feed_id,
        raw_price,
        -8,
        15_000_000, // 0.15 USD confidence
        publish_time,
        30,
        Some(250_100_000),
    )
    .expect("Failed to create sample MarketPriceUpdate")
}

fn setup_test_app() -> (axum::Router, Arc<MarketDataStore>) {
    let config = Config::from_env();
    let pool = create_db_pool(&config.database_url).expect("Failed to initialize test pool");
    let store = Arc::new(MarketDataStore::new());
    let state =
        Arc::new(AppState::new(config, pool, None, None).with_market_data_store(store.clone()));
    let router = create_router(state);
    (router, store)
}

#[tokio::test]
async fn test_rest_get_market_data_by_canonical_asset_id() {
    let (app, store) = setup_test_app();
    let now = Utc::now().timestamp();

    // Ingest price for AAPL
    let update = sample_update(
        "backed:AAPLx",
        "AAPL",
        "AAPL111111111111111111111111111111111111111",
        "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
        22_550_000_000, // $225.50
        now,
    );
    store.update(update).await.expect("Failed to store update");

    // Query GET /market-data/backed:AAPLx
    let req = Request::builder()
        .uri("/market-data/backed:AAPLx")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.expect("Request failed");
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let data: MarketDataResponse =
        serde_json::from_slice(&body_bytes).expect("Failed to deserialize MarketDataResponse");

    assert_eq!(data.asset_id, "backed:AAPLx");
    assert_eq!(data.symbol, "AAPL");
    assert_eq!(
        data.mint_address,
        "AAPL111111111111111111111111111111111111111"
    );
    assert_eq!(data.price_scaled, 225_500_000);
    assert_eq!(data.price_usd, 225.50);
    assert_eq!(data.conf_scaled, 150_000);
    assert_eq!(data.conf_bps, 6);
    assert_eq!(data.publish_time, now);
    assert!(!data.is_stale);
    assert_eq!(data.freshness_status, PriceFreshness::Fresh);
    assert_eq!(data.slot, Some(250_100_000));
}

#[tokio::test]
async fn test_rest_get_market_data_by_secondary_identifiers() {
    let (app, store) = setup_test_app();
    let now = Utc::now().timestamp();

    let update = sample_update(
        "backed:MSFTx",
        "MSFT",
        "MSFTMint11111111111111111111111111111111111",
        "0x2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b",
        415_2500_0000, // $415.25
        now,
    );
    store.update(update).await.unwrap();

    // 1. Query by SPL token mint
    let req_mint = Request::builder()
        .uri("/market-data/MSFTMint11111111111111111111111111111111111")
        .body(Body::empty())
        .unwrap();
    let res_mint = app.clone().oneshot(req_mint).await.unwrap();
    assert_eq!(res_mint.status(), StatusCode::OK);
    let bytes = res_mint.into_body().collect().await.unwrap().to_bytes();
    let data: MarketDataResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(data.asset_id, "backed:MSFTx");
    assert_eq!(data.price_scaled, 415_250_000);

    // 2. Query by ticker symbol (lowercase)
    let req_sym = Request::builder()
        .uri("/market-data/msft")
        .body(Body::empty())
        .unwrap();
    let res_sym = app.clone().oneshot(req_sym).await.unwrap();
    assert_eq!(res_sym.status(), StatusCode::OK);
    let bytes = res_sym.into_body().collect().await.unwrap().to_bytes();
    let data_sym: MarketDataResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(data_sym.asset_id, "backed:MSFTx");
}

#[tokio::test]
async fn test_rest_get_market_data_not_found() {
    let (app, _store) = setup_test_app();

    let req = Request::builder()
        .uri("/market-data/nonexistent-uuid")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"]["code"], "NOT_FOUND");
    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Market data not found for asset: nonexistent-uuid"));
}

#[tokio::test]
async fn test_websocket_connect_receive_updates_and_disconnect() {
    let (app, store) = setup_test_app();

    // Bind local TCP listener and run Axum server in background task
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/market-data/ws");

    // Client connects
    let (mut ws_stream, response) = connect_async(&ws_url)
        .await
        .expect("WebSocket connection failed");
    assert_eq!(response.status().as_u16(), 101);

    // Ingest price update into store
    let now = Utc::now().timestamp();
    let update = sample_update(
        "backed:NVDAx",
        "NVDA",
        "NVDAMint11111111111111111111111111111111111",
        "0x5feedNVDA",
        125_7500_0000, // $125.75
        now,
    );
    store.update(update).await.unwrap();

    // Client receives update
    let msg = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
        .await
        .expect("Timed out waiting for WebSocket message")
        .expect("Stream ended prematurely")
        .expect("WebSocket protocol error");

    if let Message::Text(text) = msg {
        let payload: MarketDataResponse = serde_json::from_str(&text)
            .expect("Failed to parse incoming WebSocket message as MarketDataResponse");
        assert_eq!(payload.asset_id, "backed:NVDAx");
        assert_eq!(payload.symbol, "NVDA");
        assert_eq!(payload.price_scaled, 125_750_000);
        assert_eq!(payload.price_usd, 125.75);
    } else {
        panic!("Expected text frame, got: {:?}", msg);
    }

    // Invariant: Frontend input MUST NOT alter Pyth subscriptions or store state
    // Send arbitrary client commands / subscription attempts
    ws_stream
        .send(Message::Text(
            r#"{"action":"subscribe","pyth_feed_id":"0xMALICIOUS"}"#.to_string(),
        ))
        .await
        .expect("Failed to send client message");

    // Send a second valid update to prove socket remains functional and uncompromised
    let update2 = sample_update(
        "backed:NVDAx",
        "NVDA",
        "NVDAMint11111111111111111111111111111111111",
        "0x5feedNVDA",
        126_0000_0000, // $126.00
        now + 1,
    );
    store.update(update2).await.unwrap();

    let msg2 = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
        .await
        .expect("Timed out waiting for second WebSocket message")
        .expect("Stream ended")
        .expect("WebSocket error");

    if let Message::Text(text2) = msg2 {
        let payload2: MarketDataResponse = serde_json::from_str(&text2).unwrap();
        assert_eq!(payload2.price_scaled, 126_000_000);
    } else {
        panic!("Expected text frame for second update");
    }

    // Client disconnects cleanly
    ws_stream.close(None).await.expect("Failed to close socket");

    // Subsequent updates to store do NOT crash the server
    store
        .update(sample_update(
            "backed:NVDAx",
            "NVDA",
            "MINT",
            "FEED",
            127_0000_0000,
            now + 2,
        ))
        .await
        .unwrap();

    server_handle.abort();
}

#[tokio::test]
async fn test_websocket_slow_client_does_not_block_ingestion_or_other_clients() {
    let (app, store) = setup_test_app();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/market-data/ws");

    // 1. Connect Fast Client
    let (mut fast_client, _) = connect_async(&ws_url).await.unwrap();

    // 2. Connect Slow Client (does not read from socket)
    let (_slow_client, _) = connect_async(&ws_url).await.unwrap();

    let now = Utc::now().timestamp();

    // 3. Ingest updates rapidly
    for i in 1..=10 {
        let update = sample_update(
            "backed:TSLAx",
            "TSLA",
            "MINT_TSLA",
            "FEED_TSLA",
            (200 + i) * 1_0000_0000,
            now + i,
        );
        // Ingestion must complete immediately without waiting on slow_client
        store.update(update).await.unwrap();
    }

    // 4. Fast client should receive all 10 updates cleanly
    for i in 1..=10 {
        let msg = tokio::time::timeout(Duration::from_secs(2), fast_client.next())
            .await
            .expect("Timed out waiting for fast client message")
            .unwrap()
            .unwrap();

        if let Message::Text(text) = msg {
            let res: MarketDataResponse = serde_json::from_str(&text).unwrap();
            assert_eq!(res.price_scaled, (200 + i as u64) * 1_000_000);
        }
    }

    server_handle.abort();
}

#[tokio::test]
async fn test_phase8_acceptance_criteria_frontend_observes_validated_prices_without_pyth() {
    // ACCEPTANCE CRITERIA:
    // A frontend client can observe validated live market prices without directly accessing Pyth.

    let (app, store) = setup_test_app();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 1. Backend ingests a validated Pyth market price
    let canonical_asset_id = "018f9e2b-7c1a-7b3e-9f3a-123456789abc";
    let now = Utc::now().timestamp();

    let validated_pyth_update = sample_update(
        canonical_asset_id,
        "SPY",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
        520_4500_0000, // $520.45
        now,
    );
    store.update(validated_pyth_update).await.unwrap();

    // 2. Frontend Client queries REST API: GET /market-data/:asset_id
    let client = reqwest::Client::new();
    let rest_url = format!("http://{addr}/market-data/{canonical_asset_id}");
    let response = client
        .get(&rest_url)
        .send()
        .await
        .expect("Frontend REST request to market data endpoint failed");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let rest_data: MarketDataResponse = response.json().await.unwrap();
    assert_eq!(rest_data.asset_id, canonical_asset_id);
    assert_eq!(rest_data.symbol, "SPY");
    assert_eq!(rest_data.price_scaled, 520_450_000);
    assert_eq!(rest_data.price_usd, 520.45);
    assert_eq!(rest_data.freshness_status, PriceFreshness::Fresh);

    // 3. Frontend Client connects to real-time WebSocket: GET /market-data/ws
    let ws_url = format!("ws://{addr}/market-data/ws");
    let (mut ws_stream, _) = connect_async(&ws_url).await.unwrap();

    // 4. Backend produces a new price tick
    let next_tick = sample_update(
        canonical_asset_id,
        "SPY",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
        520_8500_0000, // $520.85
        now + 1,
    );
    store.update(next_tick).await.unwrap();

    // 5. Frontend Client receives the live stream event
    let frame = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
        .await
        .expect("Timeout waiting for live market stream frame")
        .unwrap()
        .unwrap();

    if let Message::Text(json_payload) = frame {
        let live_update: MarketDataResponse = serde_json::from_str(&json_payload).unwrap();
        assert_eq!(live_update.asset_id, canonical_asset_id);
        assert_eq!(live_update.price_scaled, 520_850_000);
        assert_eq!(live_update.price_usd, 520.85);
        assert_eq!(live_update.freshness_status, PriceFreshness::Fresh);
    } else {
        panic!("Expected text frame on market data stream");
    }

    server_handle.abort();
}
