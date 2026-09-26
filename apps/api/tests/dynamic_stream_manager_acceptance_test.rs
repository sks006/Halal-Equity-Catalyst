//! Full end-to-end acceptance criteria test for Phase 5:
//! Asset Registry -> AssetMarketDataRepository -> AssetSubscriptionWatcher
//! -> watch::Receiver<SubscriptionSet> -> DynamicStreamManager -> Pyth SSE -> price updates.

use equity_catalyst_api::{
    models::canonical_asset::CreateAssetRequest,
    repositories::{AssetMarketDataRepository, CanonicalAssetRepository},
    services::asset_subscription_watcher::{AssetSubscriptionWatcher, SubscriptionWatcherConfig},
};
use equity_catalyst_pyth::{
    known_feeds, DynamicStreamManager, EnrichedPriceUpdate, StreamManagerConfig,
};
use equity_catalyst_shared::AssetApprovalStatus;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};

fn sample_asset_request(
    asset_id: &str,
    symbol: &str,
    mint: &str,
    status: AssetApprovalStatus,
) -> CreateAssetRequest {
    CreateAssetRequest {
        asset_id: asset_id.to_string(),
        symbol: symbol.to_string(),
        mint_address: mint.to_string(),
        legal_issuer: "Backed Finance AG".to_string(),
        custodian: "Maerki Baumann & Co. AG".to_string(),
        underlying_asset_identifier: format!("NASDAQ:{} (ISIN US0000000000)", symbol),
        is_active: false,
        approval_status: Some(status),
        decimals: 8,
    }
}

const AAPL_FEED: &str = known_feeds::AAPL_USD;
const TSLA_FEED: &str = known_feeds::TSLA_USD;
const NVDA_FEED: &str = known_feeds::NVDA_USD;

fn sse_event(feed_id: &str, price: &str) -> String {
    format!(
        "event: price_update\n\
        data: {{\"parsed\":[{{\"id\":\"{}\",\"price\":{{\"price\":\"{}\",\"conf\":\"10000000\",\"expo\":-8,\"publish_time\":1700000000}}}}]}}\n\n",
        feed_id, price
    )
}

/// Acceptance Criteria Test for Phase 5:
/// Starting state:
///   AAPL
///   TSLA
///
/// Then database changes to:
///   AAPL
///   TSLA
///   NVDA
///
/// The running process automatically changes the Pyth subscription to:
///   AAPL
///   TSLA
///   NVDA
/// without restarting the application.
#[tokio::test]
async fn test_phase_5_end_to_end_acceptance_criteria() {
    // 1. Initialize mock Pyth Hermes SSE server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let hermes_url = format!("http://{}", local_addr);

    let connection_count = Arc::new(AtomicUsize::new(0));
    let cc = connection_count.clone();

    tokio::spawn(async move {
        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n";

        // Session 1: Client connects with initial feeds [AAPL, TSLA]
        let (mut socket1, _) = listener.accept().await.unwrap();
        cc.fetch_add(1, Ordering::SeqCst);
        let mut buf1 = [0u8; 1024];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8_lossy(&buf1[..n1]);

        assert!(req1.contains(AAPL_FEED), "Session 1 must subscribe to AAPL");
        assert!(req1.contains(TSLA_FEED), "Session 1 must subscribe to TSLA");
        assert!(
            !req1.contains(NVDA_FEED),
            "Session 1 must NOT subscribe to NVDA yet"
        );

        socket1.write_all(headers.as_bytes()).await.unwrap();
        socket1
            .write_all(sse_event(AAPL_FEED, "22500000000").as_bytes())
            .await
            .unwrap();
        socket1.flush().await.unwrap();

        // Session 2: Client reconnects dynamically after database adds NVDA [AAPL, TSLA, NVDA]
        let (mut socket2, _) = listener.accept().await.unwrap();
        cc.fetch_add(1, Ordering::SeqCst);
        let mut buf2 = [0u8; 1024];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8_lossy(&buf2[..n2]);

        assert!(req2.contains(AAPL_FEED), "Session 2 must subscribe to AAPL");
        assert!(req2.contains(TSLA_FEED), "Session 2 must subscribe to TSLA");
        assert!(
            req2.contains(NVDA_FEED),
            "Session 2 must subscribe to newly added NVDA"
        );

        socket2.write_all(headers.as_bytes()).await.unwrap();
        socket2
            .write_all(sse_event(NVDA_FEED, "12500000000").as_bytes())
            .await
            .unwrap();
        socket2.flush().await.unwrap();
    });

    // 2. Initialize Repositories (PostgreSQL abstraction)
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    // 3. Seed Starting state in database: AAPL and TSLA
    for (id, sym, mint, feed) in [
        ("backed:AAPLx", "AAPL", "MintAAPL", AAPL_FEED),
        ("backed:TSLAx", "TSLA", "MintTSLA", TSLA_FEED),
    ] {
        let req = sample_asset_request(id, sym, mint, AssetApprovalStatus::ShariahApproved);
        asset_repo.create_asset(&req).await.unwrap();
        asset_repo.activate_asset(id).await.unwrap();
        market_repo.create_mapping(id, feed).await.unwrap();
    }

    // 4. Initialize AssetSubscriptionWatcher (Phase 3)
    let watcher_config = SubscriptionWatcherConfig::new(Duration::from_millis(25));
    let watcher = Arc::new(
        AssetSubscriptionWatcher::new_initialized(market_repo.clone(), watcher_config)
            .await
            .unwrap(),
    );

    let (shutdown_tx, shutdown_rx_watcher) = broadcast::channel(1);
    let shutdown_rx_manager = shutdown_tx.subscribe();
    let watcher_handle = watcher.clone().start(shutdown_rx_watcher);

    // 5. Initialize DynamicStreamManager (Phase 5)
    let (price_tx, mut price_rx) = mpsc::channel(10);
    let manager_config = StreamManagerConfig::new(hermes_url).with_backoff(
        Duration::from_millis(20),
        Duration::from_millis(100),
        2.0,
    );

    let manager = DynamicStreamManager::with_channel(manager_config, watcher.subscribe(), price_tx);
    let manager_handle = manager.spawn(shutdown_rx_manager);

    // 6. Verify State 1: Receives AAPL price update from Session 1
    let event_1 = tokio::time::timeout(Duration::from_millis(500), price_rx.recv())
        .await
        .expect("Timeout waiting for State 1 update")
        .expect("Price channel open");

    assert_eq!(event_1.feeds().len(), 1);
    assert_eq!(event_1.feeds()[0].id, AAPL_FEED);
    assert_eq!(event_1.feeds()[0].price.price, "22500000000");

    // 7. Mutate Database at runtime: Add NVDA (zero code changes, no process restart)
    let nvda_req = sample_asset_request(
        "backed:NVDAx",
        "NVDA",
        "MintNVDA",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&nvda_req).await.unwrap();
    asset_repo.activate_asset("backed:NVDAx").await.unwrap();
    market_repo
        .create_mapping("backed:NVDAx", NVDA_FEED)
        .await
        .unwrap();

    // 8. Verify State 2: Watcher detects NVDA -> Manager drops Session 1 -> Reconnects Session 2
    // and receives NVDA price update
    let event_2 = tokio::time::timeout(Duration::from_millis(1000), price_rx.recv())
        .await
        .expect("Timeout waiting for State 2 update after dynamic DB mutation")
        .expect("Price channel open");

    assert_eq!(event_2.feeds().len(), 1);
    assert_eq!(event_2.feeds()[0].id, NVDA_FEED);
    assert_eq!(event_2.feeds()[0].price.price, "12500000000");

    // Verify 2 sessions were created on the mock server
    assert_eq!(connection_count.load(Ordering::SeqCst), 2);

    // 9. Clean shutdown
    shutdown_tx.send(()).unwrap();
    let _ = watcher_handle.await;
    let _ = manager_handle.await;
}

/// Acceptance Criteria Test for Phase P3:
/// Database:
///   AAPL
///   TSLA
/// produces Pyth subscription = AAPL + TSLA.
///
/// Every update preserves:
///   * Pyth feed ID
///   * asset ID
///   * mint address
///   * publish time
///   * confidence
///   * price
///
/// Database changes to:
///   AAPL
///   TSLA
///   NVDA
///
/// The live process automatically reconnects/subscribes to:
///   AAPL + TSLA + NVDA
/// No restart required.
#[tokio::test]
async fn test_phase_p3_acceptance_criteria_with_enriched_updates() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let hermes_url = format!("http://{}", local_addr);

    let connection_count = Arc::new(AtomicUsize::new(0));
    let cc = connection_count.clone();

    tokio::spawn(async move {
        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n";

        // Session 1: Client connects with initial feeds [AAPL, TSLA]
        let (mut socket1, _) = listener.accept().await.unwrap();
        cc.fetch_add(1, Ordering::SeqCst);
        let mut buf1 = [0u8; 1024];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8_lossy(&buf1[..n1]);

        assert!(req1.contains(AAPL_FEED), "Session 1 must subscribe to AAPL");
        assert!(req1.contains(TSLA_FEED), "Session 1 must subscribe to TSLA");
        assert!(
            !req1.contains(NVDA_FEED),
            "Session 1 must NOT subscribe to NVDA yet"
        );

        socket1.write_all(headers.as_bytes()).await.unwrap();
        socket1
            .write_all(sse_event(AAPL_FEED, "22550000000").as_bytes())
            .await
            .unwrap();
        socket1.flush().await.unwrap();

        // Session 2: Client reconnects dynamically after database adds NVDA [AAPL, TSLA, NVDA]
        let (mut socket2, _) = listener.accept().await.unwrap();
        cc.fetch_add(1, Ordering::SeqCst);
        let mut buf2 = [0u8; 1024];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8_lossy(&buf2[..n2]);

        assert!(req2.contains(AAPL_FEED), "Session 2 must subscribe to AAPL");
        assert!(req2.contains(TSLA_FEED), "Session 2 must subscribe to TSLA");
        assert!(
            req2.contains(NVDA_FEED),
            "Session 2 must subscribe to newly added NVDA"
        );

        socket2.write_all(headers.as_bytes()).await.unwrap();
        socket2
            .write_all(sse_event(NVDA_FEED, "12575000000").as_bytes())
            .await
            .unwrap();
        socket2.flush().await.unwrap();
    });

    // 1. Initialize Repositories (PostgreSQL abstraction)
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    // 2. Starting database state: AAPL and TSLA
    for (id, sym, mint, feed) in [
        (
            "backed:AAPLx",
            "AAPL",
            "MintAAPL111111111111111111111111111111111",
            AAPL_FEED,
        ),
        (
            "backed:TSLAx",
            "TSLA",
            "MintTSLA111111111111111111111111111111111",
            TSLA_FEED,
        ),
    ] {
        let req = sample_asset_request(id, sym, mint, AssetApprovalStatus::ShariahApproved);
        asset_repo.create_asset(&req).await.unwrap();
        asset_repo.activate_asset(id).await.unwrap();
        market_repo.create_mapping(id, feed).await.unwrap();
    }

    // 3. Subscription Watcher from DB
    let watcher_config = SubscriptionWatcherConfig::new(Duration::from_millis(25));
    let watcher = Arc::new(
        AssetSubscriptionWatcher::new_initialized(market_repo.clone(), watcher_config)
            .await
            .unwrap(),
    );

    let (shutdown_tx, shutdown_rx_watcher) = broadcast::channel(1);
    let shutdown_rx_manager = shutdown_tx.subscribe();
    let watcher_handle = watcher.clone().start(shutdown_rx_watcher);

    // 4. DynamicStreamManager with enriched channel preserving all 6 fields
    let (enriched_tx, mut enriched_rx) = mpsc::channel::<EnrichedPriceUpdate>(10);
    let manager_config = StreamManagerConfig::new(hermes_url).with_backoff(
        Duration::from_millis(20),
        Duration::from_millis(100),
        2.0,
    );

    let manager = DynamicStreamManager::with_enriched_channel(
        manager_config,
        watcher.subscribe(),
        enriched_tx,
    );
    let manager_handle = manager.spawn(shutdown_rx_manager);

    // 5. Verify State 1: Enriched AAPL update preserves all 6 fields
    let update_1 = tokio::time::timeout(Duration::from_millis(500), enriched_rx.recv())
        .await
        .expect("Timeout waiting for State 1 update")
        .expect("Price channel open");

    assert_eq!(update_1.pyth_feed_id, AAPL_FEED, "Preserves Pyth feed ID");
    assert_eq!(update_1.asset_id, "backed:AAPLx", "Preserves asset ID");
    assert_eq!(
        update_1.mint_address, "MintAAPL111111111111111111111111111111111",
        "Preserves mint address"
    );
    assert_eq!(update_1.publish_time, 1700000000, "Preserves publish time");
    assert_eq!(update_1.confidence, "10000000", "Preserves confidence");
    assert_eq!(update_1.price, "22550000000", "Preserves price");

    // 6. Mutate Database at runtime: Add NVDA (zero code changes, no process restart)
    let nvda_req = sample_asset_request(
        "backed:NVDAx",
        "NVDA",
        "MintNVDA111111111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&nvda_req).await.unwrap();
    asset_repo.activate_asset("backed:NVDAx").await.unwrap();
    market_repo
        .create_mapping("backed:NVDAx", NVDA_FEED)
        .await
        .unwrap();

    // 7. Verify State 2: Watcher detects NVDA -> Manager drops Session 1 -> Reconnects Session 2
    // and receives NVDA price update preserving all 6 fields
    let update_2 = tokio::time::timeout(Duration::from_millis(1000), enriched_rx.recv())
        .await
        .expect("Timeout waiting for State 2 update after dynamic DB mutation")
        .expect("Price channel open");

    assert_eq!(update_2.pyth_feed_id, NVDA_FEED, "Preserves NVDA feed ID");
    assert_eq!(update_2.asset_id, "backed:NVDAx", "Preserves NVDA asset ID");
    assert_eq!(
        update_2.mint_address, "MintNVDA111111111111111111111111111111111",
        "Preserves NVDA mint address"
    );
    assert_eq!(update_2.publish_time, 1700000000, "Preserves publish time");
    assert_eq!(update_2.confidence, "10000000", "Preserves confidence");
    assert_eq!(update_2.price, "12575000000", "Preserves price");

    // Verify 2 sessions were created on the mock server
    assert_eq!(connection_count.load(Ordering::SeqCst), 2);

    // 8. Clean shutdown
    shutdown_tx.send(()).unwrap();
    let _ = watcher_handle.await;
    let _ = manager_handle.await;
}
