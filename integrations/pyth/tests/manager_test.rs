//! Integration test suite for DynamicStreamManager.

use equity_catalyst_pyth::{
    known_feeds, DynamicStreamManager, PythSubscription, StreamManagerConfig, SubscriptionSet,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, watch};

fn make_sub(id: &str, sym: &str, feed: &str) -> PythSubscription {
    PythSubscription::new(id, sym, format!("Mint{}", sym), feed)
}

fn sse_event(feed_id: &str, price: &str) -> String {
    format!(
        "event: price_update\n\
        data: {{\"parsed\":[{{\"id\":\"{}\",\"price\":{{\"price\":\"{}\",\"conf\":\"10000000\",\"expo\":-8,\"publish_time\":1700000000}}}}]}}\n\n",
        feed_id, price
    )
}

/// Task 10: Initial subscription test.
#[tokio::test]
async fn test_manager_initial_subscription() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    // Initial subscription: AAPL and TSLA
    let sub_aapl = make_sub("backed:AAPLx", "AAPL", known_feeds::AAPL_USD);
    let sub_tsla = make_sub("backed:TSLAx", "TSLA", known_feeds::TSLA_USD);
    let initial_set = SubscriptionSet::new(vec![sub_aapl, sub_tsla]);
    let (_sub_tx, sub_rx) = watch::channel(initial_set);

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);

        assert!(req.contains(&format!("ids[]={}", known_feeds::AAPL_USD)));
        assert!(req.contains(&format!("ids[]={}", known_feeds::TSLA_USD)));

        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
        socket.write_all(headers.as_bytes()).await.unwrap();
        socket
            .write_all(sse_event(known_feeds::AAPL_USD, "22500000000").as_bytes())
            .await
            .unwrap();
        socket.flush().await.unwrap();
    });

    let config = StreamManagerConfig::new(format!("http://{}", addr));
    let manager = DynamicStreamManager::with_channel(config, sub_rx, event_tx);
    let handle = manager.spawn(shutdown_rx);

    let event = tokio::time::timeout(Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Should receive event")
        .expect("Channel open");

    assert_eq!(event.feeds().len(), 1);
    assert_eq!(event.feeds()[0].id, known_feeds::AAPL_USD);
    assert_eq!(event.feeds()[0].price.price, "22500000000");

    let _ = shutdown_tx.send(());
    let _ = handle.await;
}

/// Task 10: Adding an asset triggers dynamic reconnection with expanded feed list.
#[tokio::test]
async fn test_manager_adding_an_asset() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    // Initial subscription: AAPL and TSLA
    let sub_aapl = make_sub("backed:AAPLx", "AAPL", known_feeds::AAPL_USD);
    let sub_tsla = make_sub("backed:TSLAx", "TSLA", known_feeds::TSLA_USD);
    let initial_set = SubscriptionSet::new(vec![sub_aapl.clone(), sub_tsla.clone()]);
    let (sub_tx, sub_rx) = watch::channel(initial_set);

    let connection_count = Arc::new(AtomicUsize::new(0));
    let cc = connection_count.clone();

    tokio::spawn(async move {
        // Connection 1: AAPL and TSLA
        let (mut socket1, _) = listener.accept().await.unwrap();
        cc.fetch_add(1, Ordering::SeqCst);
        let mut buf = [0u8; 1024];
        let n = socket1.read(&mut buf).await.unwrap();
        let req1 = String::from_utf8_lossy(&buf[..n]);
        assert!(!req1.contains(known_feeds::NVDA_USD));

        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n";
        socket1.write_all(headers.as_bytes()).await.unwrap();
        socket1
            .write_all(sse_event(known_feeds::AAPL_USD, "22500000000").as_bytes())
            .await
            .unwrap();
        socket1.flush().await.unwrap();

        // Connection 2: AAPL, TSLA, and NVDA
        let (mut socket2, _) = listener.accept().await.unwrap();
        cc.fetch_add(1, Ordering::SeqCst);
        let mut buf2 = [0u8; 1024];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8_lossy(&buf2[..n2]);
        assert!(req2.contains(known_feeds::NVDA_USD));

        socket2.write_all(headers.as_bytes()).await.unwrap();
        socket2
            .write_all(sse_event(known_feeds::NVDA_USD, "12000000000").as_bytes())
            .await
            .unwrap();
        socket2.flush().await.unwrap();
    });

    let config = StreamManagerConfig::new(format!("http://{}", addr));
    let manager = DynamicStreamManager::with_channel(config, sub_rx, event_tx);
    let handle = manager.spawn(shutdown_rx);

    // Receive first update from initial subscription
    let ev1 = event_rx.recv().await.unwrap();
    assert_eq!(ev1.feeds()[0].id, known_feeds::AAPL_USD);

    // Dynamically update subscription to add NVDA
    let sub_nvda = make_sub("backed:NVDAx", "NVDA", known_feeds::NVDA_USD);
    let updated_set = SubscriptionSet::new(vec![sub_aapl, sub_tsla, sub_nvda]);
    sub_tx.send(updated_set).unwrap();

    // Receive second update from newly connected stream with NVDA
    let ev2 = tokio::time::timeout(Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Should receive update from reconnected stream")
        .expect("Channel open");

    assert_eq!(ev2.feeds()[0].id, known_feeds::NVDA_USD);
    assert_eq!(connection_count.load(Ordering::SeqCst), 2);

    let _ = shutdown_tx.send(());
    let _ = handle.await;
}

/// Task 10: Removing an asset triggers reconnection with narrowed feed list.
#[tokio::test]
async fn test_manager_removing_an_asset() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    let sub_aapl = make_sub("backed:AAPLx", "AAPL", known_feeds::AAPL_USD);
    let sub_tsla = make_sub("backed:TSLAx", "TSLA", known_feeds::TSLA_USD);
    let initial_set = SubscriptionSet::new(vec![sub_aapl.clone(), sub_tsla]);
    let (sub_tx, sub_rx) = watch::channel(initial_set);

    tokio::spawn(async move {
        // Connection 1
        let (mut socket1, _) = listener.accept().await.unwrap();
        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n";
        socket1.write_all(headers.as_bytes()).await.unwrap();
        socket1
            .write_all(sse_event(known_feeds::AAPL_USD, "220").as_bytes())
            .await
            .unwrap();
        socket1.flush().await.unwrap();

        // Connection 2 after removal of TSLA
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let n = socket2.read(&mut buf).await.unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);

        assert!(req.contains(known_feeds::AAPL_USD));
        assert!(
            !req.contains(known_feeds::TSLA_USD),
            "TSLA must be excluded after removal"
        );

        socket2.write_all(headers.as_bytes()).await.unwrap();
        socket2
            .write_all(sse_event(known_feeds::AAPL_USD, "225").as_bytes())
            .await
            .unwrap();
        socket2.flush().await.unwrap();
    });

    let config = StreamManagerConfig::new(format!("http://{}", addr));
    let manager = DynamicStreamManager::with_channel(config, sub_rx, event_tx);
    let handle = manager.spawn(shutdown_rx);

    let ev1 = event_rx.recv().await.unwrap();
    assert_eq!(ev1.feeds()[0].price.price, "220");

    // Remove TSLA
    let narrowed_set = SubscriptionSet::new(vec![sub_aapl]);
    sub_tx.send(narrowed_set).unwrap();

    let ev2 = tokio::time::timeout(Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Should receive second event")
        .expect("Channel open");

    assert_eq!(ev2.feeds()[0].price.price, "225");

    let _ = shutdown_tx.send(());
    let _ = handle.await;
}

/// Task 10: Replacing a feed ID triggers reconnection with replacement feed ID.
#[tokio::test]
async fn test_manager_replacing_a_feed() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    let old_feed = known_feeds::AAPL_USD;
    let new_feed = known_feeds::MSFT_USD;

    let initial_set = SubscriptionSet::new(vec![make_sub("backed:AAPLx", "AAPL", old_feed)]);
    let (sub_tx, sub_rx) = watch::channel(initial_set);

    tokio::spawn(async move {
        // First connection with old_feed
        let (mut socket1, _) = listener.accept().await.unwrap();
        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n";
        socket1.write_all(headers.as_bytes()).await.unwrap();
        socket1
            .write_all(sse_event(old_feed, "100").as_bytes())
            .await
            .unwrap();

        // Second connection with new_feed
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let n = socket2.read(&mut buf).await.unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);

        assert!(!req.contains(old_feed), "Old feed must not be present");
        assert!(req.contains(new_feed), "New feed must be present");

        socket2.write_all(headers.as_bytes()).await.unwrap();
        socket2
            .write_all(sse_event(new_feed, "200").as_bytes())
            .await
            .unwrap();
    });

    let config = StreamManagerConfig::new(format!("http://{}", addr));
    let manager = DynamicStreamManager::with_channel(config, sub_rx, event_tx);
    let handle = manager.spawn(shutdown_rx);

    let ev1 = event_rx.recv().await.unwrap();
    assert_eq!(ev1.feeds()[0].id, old_feed);

    // Replace feed ID for AAPL
    let replaced_set = SubscriptionSet::new(vec![make_sub("backed:AAPLx", "AAPL", new_feed)]);
    sub_tx.send(replaced_set).unwrap();

    let ev2 = tokio::time::timeout(Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Should receive event from new feed")
        .expect("Channel open");

    assert_eq!(ev2.feeds()[0].id, new_feed);

    let _ = shutdown_tx.send(());
    let _ = handle.await;
}

/// Task 10 & 5: Empty subscription: does not open stream, idles, connects when added.
#[tokio::test]
async fn test_manager_empty_subscription_idling() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    // Start with empty SubscriptionSet
    let (sub_tx, sub_rx) = watch::channel(SubscriptionSet::empty());

    let server_connected = Arc::new(AtomicUsize::new(0));
    let sc = server_connected.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        sc.fetch_add(1, Ordering::SeqCst);
        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n";
        socket.write_all(headers.as_bytes()).await.unwrap();
        socket
            .write_all(sse_event(known_feeds::AAPL_USD, "225").as_bytes())
            .await
            .unwrap();
    });

    let config = StreamManagerConfig::new(format!("http://{}", addr));
    let manager = DynamicStreamManager::with_channel(config, sub_rx, event_tx);
    let handle = manager.spawn(shutdown_rx);

    // Wait 50ms: server must NOT have been connected to while empty
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        server_connected.load(Ordering::SeqCst),
        0,
        "No connection when subscription is empty"
    );

    // Add subscription
    let set = SubscriptionSet::new(vec![make_sub(
        "backed:AAPLx",
        "AAPL",
        known_feeds::AAPL_USD,
    )]);
    sub_tx.send(set).unwrap();

    // Now connection happens and event is received
    let ev = tokio::time::timeout(Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Should receive event after subscription added")
        .expect("Channel open");

    assert_eq!(ev.feeds()[0].id, known_feeds::AAPL_USD);
    assert_eq!(server_connected.load(Ordering::SeqCst), 1);

    let _ = shutdown_tx.send(());
    let _ = handle.await;
}

/// Task 10, 6 & 7: Pyth disconnect: reconnects using current SubscriptionSet with bounded backoff.
#[tokio::test]
async fn test_manager_pyth_disconnect_and_reconnect() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    let sub_aapl = make_sub("backed:AAPLx", "AAPL", known_feeds::AAPL_USD);
    let initial_set = SubscriptionSet::new(vec![sub_aapl]);
    let (_sub_tx, sub_rx) = watch::channel(initial_set);

    tokio::spawn(async move {
        // Session 1: sends event, then abruptly drops connection (EOF)
        let (mut socket1, _) = listener.accept().await.unwrap();
        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
        socket1.write_all(headers.as_bytes()).await.unwrap();
        socket1
            .write_all(sse_event(known_feeds::AAPL_USD, "100").as_bytes())
            .await
            .unwrap();
        socket1.flush().await.unwrap();
        drop(socket1); // Disconnect!

        // Session 2: reconnects automatically with same subscription
        let (mut socket2, _) = listener.accept().await.unwrap();
        socket2.write_all(headers.as_bytes()).await.unwrap();
        socket2
            .write_all(sse_event(known_feeds::AAPL_USD, "105").as_bytes())
            .await
            .unwrap();
        socket2.flush().await.unwrap();
    });

    // Short backoff for fast testing
    let config = StreamManagerConfig::new(format!("http://{}", addr)).with_backoff(
        Duration::from_millis(20),
        Duration::from_millis(100),
        2.0,
    );

    let manager = DynamicStreamManager::with_channel(config, sub_rx, event_tx);
    let handle = manager.spawn(shutdown_rx);

    // Event 1 from session 1
    let ev1 = event_rx.recv().await.unwrap();
    assert_eq!(ev1.feeds()[0].price.price, "100");

    // Event 2 from session 2 after automatic reconnection
    let ev2 = tokio::time::timeout(Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Should receive event 2 after reconnection")
        .expect("Channel open");

    assert_eq!(ev2.feeds()[0].price.price, "105");

    let _ = shutdown_tx.send(());
    let _ = handle.await;
}

/// Phase P3 Task 5: 24-hour stream termination handling (proactive rotation).
#[tokio::test]
async fn test_manager_24h_stream_termination_clean_rotation() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    let sub_aapl = make_sub("backed:AAPLx", "AAPL", known_feeds::AAPL_USD);
    let initial_set = SubscriptionSet::new(vec![sub_aapl]);
    let (_sub_tx, sub_rx) = watch::channel(initial_set);

    let rotation_count = Arc::new(AtomicUsize::new(0));
    let rc = rotation_count.clone();

    tokio::spawn(async move {
        // Session 1: initial connection
        let (mut socket1, _) = listener.accept().await.unwrap();
        rc.fetch_add(1, Ordering::SeqCst);
        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n";
        socket1.write_all(headers.as_bytes()).await.unwrap();
        socket1
            .write_all(sse_event(known_feeds::AAPL_USD, "200").as_bytes())
            .await
            .unwrap();
        socket1.flush().await.unwrap();

        // Session 2: connected cleanly after 24h stream duration timeout triggers rotation
        let (mut socket2, _) = listener.accept().await.unwrap();
        rc.fetch_add(1, Ordering::SeqCst);
        socket2.write_all(headers.as_bytes()).await.unwrap();
        socket2
            .write_all(sse_event(known_feeds::AAPL_USD, "205").as_bytes())
            .await
            .unwrap();
        socket2.flush().await.unwrap();
    });

    // Configure max_stream_duration to 60ms to simulate 24-hour timeout deterministically
    let config = StreamManagerConfig::new(format!("http://{}", addr))
        .with_max_stream_duration(Duration::from_millis(60))
        .with_backoff(Duration::from_millis(10), Duration::from_millis(50), 2.0);

    let manager = DynamicStreamManager::with_channel(config, sub_rx, event_tx);
    let handle = manager.spawn(shutdown_rx);

    // Event 1 from session 1
    let ev1 = event_rx.recv().await.unwrap();
    assert_eq!(ev1.feeds()[0].price.price, "200");

    // Event 2 from session 2 after 24h timeout rotation
    let ev2 = tokio::time::timeout(Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Should receive event 2 after 24h stream rotation")
        .expect("Channel open");

    assert_eq!(ev2.feeds()[0].price.price, "205");
    assert_eq!(rotation_count.load(Ordering::SeqCst), 2);

    let _ = shutdown_tx.send(());
    let _ = handle.await;
}

/// Phase P3 Task 9 & 10: Every update must preserve all 6 fields without using symbols for inference.
#[tokio::test]
async fn test_manager_enriched_updates_preserve_all_six_fields() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    let feed_id = known_feeds::AAPL_USD;
    let asset_id = "backed:AAPLx";
    let mint_address = "MintAAPLx111111111111111111111111111111111";
    let symbol = "AAPL";

    let sub = PythSubscription::new(asset_id, symbol, mint_address, feed_id);
    let initial_set = SubscriptionSet::new(vec![sub]);
    let (_sub_tx, sub_rx) = watch::channel(initial_set);

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n";
        socket.write_all(headers.as_bytes()).await.unwrap();

        // Feed update with explicit price, conf, publish_time
        let payload = format!(
            "event: price_update\n\
            data: {{\"parsed\":[{{\"id\":\"{}\",\"price\":{{\"price\":\"24550000000\",\"conf\":\"1500000\",\"expo\":-8,\"publish_time\":1710000000}}}}]}}\n\n",
            feed_id
        );
        socket.write_all(payload.as_bytes()).await.unwrap();
        socket.flush().await.unwrap();
    });

    let config = StreamManagerConfig::new(format!("http://{}", addr));
    let manager = DynamicStreamManager::with_enriched_channel(config, sub_rx, event_tx);
    let handle = manager.spawn(shutdown_rx);

    let update = tokio::time::timeout(Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Should receive enriched update")
        .expect("Channel open");

    // Verify all 6 mandatory fields preserved (Task 9)
    assert_eq!(update.pyth_feed_id, feed_id, "Preserves Pyth feed ID");
    assert_eq!(update.asset_id, asset_id, "Preserves asset ID");
    assert_eq!(update.mint_address, mint_address, "Preserves mint address");
    assert_eq!(update.publish_time, 1710000000, "Preserves publish time");
    assert_eq!(update.confidence, "1500000", "Preserves confidence");
    assert_eq!(update.price, "24550000000", "Preserves price");
    assert_eq!(update.symbol, "AAPL", "Preserves symbol metadata");

    // Math conversions
    assert_eq!(update.parse_price_raw().unwrap(), 24550000000);
    assert_eq!(update.parse_conf_raw().unwrap(), 1500000);
    assert!((update.price_usd().unwrap() - 245.50).abs() < 1e-6);

    let _ = shutdown_tx.send(());
    let _ = handle.await;
}

/// Phase P3 Task 8: Feeds not present in the approved database mapping are filtered out.
#[tokio::test]
async fn test_manager_unapproved_feeds_filtered_out() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    // Only AAPL is approved in the database subscription set
    let sub_aapl = make_sub("backed:AAPLx", "AAPL", known_feeds::AAPL_USD);
    let initial_set = SubscriptionSet::new(vec![sub_aapl]);
    let (_sub_tx, sub_rx) = watch::channel(initial_set);

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let headers =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n";
        socket.write_all(headers.as_bytes()).await.unwrap();

        // Server sends an update with two feeds: AAPL (approved) and an UNAPPROVED rogue feed
        let payload = format!(
            "event: price_update\n\
            data: {{\"parsed\":[\
                {{\"id\":\"{}\",\"price\":{{\"price\":\"111\",\"conf\":\"1\",\"expo\":-2,\"publish_time\":1000}}}},\
                {{\"id\":\"deadbeef00000000000000000000000000000000000000000000000000000000\",\"price\":{{\"price\":\"999\",\"conf\":\"1\",\"expo\":-2,\"publish_time\":1000}}}}\
            ]}}\n\n",
            known_feeds::AAPL_USD
        );
        socket.write_all(payload.as_bytes()).await.unwrap();
        socket.flush().await.unwrap();
    });

    let config = StreamManagerConfig::new(format!("http://{}", addr));
    let manager = DynamicStreamManager::with_enriched_channel(config, sub_rx, event_tx);
    let handle = manager.spawn(shutdown_rx);

    // Only AAPL is dispatched because deadbeef is not in the approved database mapping
    let update = tokio::time::timeout(Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Should receive AAPL update")
        .expect("Channel open");

    assert_eq!(update.pyth_feed_id, known_feeds::AAPL_USD);
    assert_eq!(update.asset_id, "backed:AAPLx");

    // No further updates in queue (deadbeef was filtered)
    let extra = tokio::time::timeout(Duration::from_millis(50), event_rx.recv()).await;
    assert!(extra.is_err(), "Unapproved feed must not be dispatched");

    let _ = shutdown_tx.send(());
    let _ = handle.await;
}
