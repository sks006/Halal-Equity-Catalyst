//! Integration tests for Phase 6: In-Memory Market Data Store.
//!
//! Validates:
//! - Canonical asset identity resolution (asset_id as primary key).
//! - Secondary index lookups (mint, feed_id, symbol).
//! - Deterministic integer-scaled pricing (micro-USD, 6 decimals) without f64 in execution calculations.
//! - Freshness metadata evaluation and staleness checks.
//! - Pub/sub broadcast updates.
//! - Thread safety and lock-free execution across async boundaries.
//! - Full acceptance criteria: receiving Pyth updates and retrieving by canonical asset identity.

use chrono::Utc;
use equity_catalyst_api::services::{
    calculate_scaled_int, MarketDataError, MarketDataStore, MarketPriceUpdate, PriceFreshness,
};
use equity_catalyst_pyth::{ParsedPriceFeed, PythRawPrice, PythSubscription};
use std::sync::Arc;
use std::time::Duration;

fn make_test_subscription(asset_id: &str, symbol: &str, mint: &str, feed_id: &str) -> PythSubscription {
    PythSubscription {
        asset_id: asset_id.to_string(),
        symbol: symbol.to_string(),
        mint_address: mint.to_string(),
        pyth_feed_id: feed_id.to_string(),
    }
}

fn make_test_pyth_feed(feed_id: &str, price: i64, conf: u64, expo: i32, publish_time: i64) -> ParsedPriceFeed {
    ParsedPriceFeed {
        id: feed_id.to_string(),
        price: PythRawPrice {
            price: price.to_string(),
            conf: conf.to_string(),
            expo,
            publish_time,
        },
        ema_price: None,
    }
}

#[tokio::test]
async fn test_market_data_store_insert_and_lookup() {
    let store = MarketDataStore::new();
    let sub = make_test_subscription(
        "asset-aapl-uuid-001",
        "AAPL",
        "AAPL111111111111111111111111111111111111111",
        "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
    );

    let now_ts = Utc::now().timestamp();
    // $225.50 = 22550000000 with expo -8
    let pyth_feed = make_test_pyth_feed(&sub.pyth_feed_id, 22_550_000_000, 15_000_000, -8, now_ts);

    let update = MarketPriceUpdate::from_pyth(&sub, &pyth_feed).expect("Failed to build MarketPriceUpdate");
    assert_eq!(update.asset_id, "asset-aapl-uuid-001");
    // 225.50 USD in micro-USD (6 decimals) = 225_500_000
    assert_eq!(update.price_scaled, 225_500_000);
    // 0.15 USD conf in micro-USD = 150_000
    assert_eq!(update.conf_scaled, 150_000);
    // BPS: (150_000 * 10,000) / 225_500_000 = 6 bps
    assert_eq!(update.conf_bps, 6);

    store.update(update).await.expect("Failed to store update");

    // 1. Primary lookup by canonical asset_id
    let retrieved_by_id = store.get_by_asset_id("asset-aapl-uuid-001").await;
    assert!(retrieved_by_id.is_some());
    let r1 = retrieved_by_id.unwrap();
    assert_eq!(r1.asset_id, "asset-aapl-uuid-001");
    assert_eq!(r1.symbol, "AAPL");
    assert_eq!(r1.price_scaled, 225_500_000);

    // 2. Secondary lookup by mint_address
    let retrieved_by_mint = store
        .get_by_mint("AAPL111111111111111111111111111111111111111")
        .await;
    assert!(retrieved_by_mint.is_some());
    assert_eq!(retrieved_by_mint.unwrap().asset_id, "asset-aapl-uuid-001");

    // 3. Secondary lookup by pyth_feed_id (both with and without 0x prefix)
    let retrieved_by_feed = store.get_by_feed(&sub.pyth_feed_id).await;
    assert!(retrieved_by_feed.is_some());
    assert_eq!(retrieved_by_feed.unwrap().asset_id, "asset-aapl-uuid-001");

    let clean_feed = sub.pyth_feed_id.trim_start_matches("0x");
    let retrieved_by_clean_feed = store.get_by_feed(clean_feed).await;
    assert!(retrieved_by_clean_feed.is_some());
    assert_eq!(retrieved_by_clean_feed.unwrap().asset_id, "asset-aapl-uuid-001");

    // 4. Secondary lookup by symbol (case insensitive)
    let retrieved_by_sym = store.get_by_symbol("aapl").await;
    assert!(retrieved_by_sym.is_some());
    assert_eq!(retrieved_by_sym.unwrap().asset_id, "asset-aapl-uuid-001");

    // 5. Generic get_latest resolving any of the identifiers
    let retrieved_latest = store.get_latest("asset-aapl-uuid-001").await;
    assert!(retrieved_latest.is_some());

    assert_eq!(store.len().await, 1);
    assert!(!store.is_empty().await);
}

#[tokio::test]
async fn test_market_data_store_update_overwrites_older_data() {
    let store = MarketDataStore::new();
    let sub = make_test_subscription(
        "asset-msft-uuid-002",
        "MSFT",
        "MSFT111111111111111111111111111111111111111",
        "0x2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b",
    );

    let ts1 = 1700000000;
    let feed1 = make_test_pyth_feed(&sub.pyth_feed_id, 400_0000_0000, 20_0000, -8, ts1);
    let update1 = MarketPriceUpdate::from_pyth(&sub, &feed1).unwrap();
    store.update(update1).await.unwrap();

    let fetched1 = store.get_by_asset_id(&sub.asset_id).await.unwrap();
    assert_eq!(fetched1.price_scaled, 400_000_000); // $400.00
    assert_eq!(fetched1.publish_time, ts1);

    // Update with newer price: $405.25
    let ts2 = 1700000010;
    let feed2 = make_test_pyth_feed(&sub.pyth_feed_id, 405_2500_0000, 25_0000, -8, ts2);
    let update2 = MarketPriceUpdate::from_pyth(&sub, &feed2).unwrap();
    store.update(update2).await.unwrap();

    let fetched2 = store.get_by_asset_id(&sub.asset_id).await.unwrap();
    assert_eq!(fetched2.price_scaled, 405_250_000); // $405.25
    assert_eq!(fetched2.publish_time, ts2);
    assert_eq!(store.len().await, 1);
}

#[tokio::test]
async fn test_market_data_store_unknown_asset_returns_none() {
    let store = MarketDataStore::new();
    assert!(store.get_latest("non-existent-uuid").await.is_none());
    assert!(store.get_by_asset_id("non-existent-uuid").await.is_none());
    assert!(store.get_by_mint("non-existent-mint").await.is_none());
    assert!(store.get_by_feed("0x0000000000000000").await.is_none());
    assert!(store.get_by_symbol("UNKNOWN").await.is_none());
    assert!(!store.contains_asset("non-existent-uuid").await);
}

#[tokio::test]
async fn test_market_data_store_get_all() {
    let store = MarketDataStore::new();

    let sub1 = make_test_subscription("asset-1", "AAPL", "MINT1", "0xFEED1");
    let sub2 = make_test_subscription("asset-2", "GOOGL", "MINT2", "0xFEED2");
    let sub3 = make_test_subscription("asset-3", "TSLA", "MINT3", "0xFEED3");

    let f1 = make_test_pyth_feed("0xFEED1", 150_0000_0000, 10_0000, -8, 1700000000);
    let f2 = make_test_pyth_feed("0xFEED2", 175_0000_0000, 10_0000, -8, 1700000000);
    let f3 = make_test_pyth_feed("0xFEED3", 250_0000_0000, 10_0000, -8, 1700000000);

    store.update(MarketPriceUpdate::from_pyth(&sub1, &f1).unwrap()).await.unwrap();
    store.update(MarketPriceUpdate::from_pyth(&sub2, &f2).unwrap()).await.unwrap();
    store.update(MarketPriceUpdate::from_pyth(&sub3, &f3).unwrap()).await.unwrap();

    let all = store.get_all().await;
    assert_eq!(all.len(), 3);

    let mut asset_ids: Vec<String> = all.into_iter().map(|p| p.asset_id).collect();
    asset_ids.sort();
    assert_eq!(asset_ids, vec!["asset-1", "asset-2", "asset-3"]);
}

#[tokio::test]
async fn test_market_data_store_subscribe_broadcast() {
    let store = MarketDataStore::new();
    let mut rx1 = store.subscribe();
    let mut rx2 = store.subscribe();

    let sub = make_test_subscription("asset-nvda", "NVDA", "MINT_NVDA", "0xFEEDNVDA");
    let feed = make_test_pyth_feed("0xFEEDNVDA", 120_0000_0000, 5_0000, -8, 1700000000);
    let update = MarketPriceUpdate::from_pyth(&sub, &feed).unwrap();

    store.update(update.clone()).await.unwrap();

    // Verify both subscribers received the notification
    let msg1 = tokio::time::timeout(Duration::from_millis(500), rx1.recv())
        .await
        .expect("Timed out waiting for rx1")
        .expect("rx1 receive failed");
    assert_eq!(msg1.asset_id, "asset-nvda");
    assert_eq!(msg1.price_scaled, 120_000_000);

    let msg2 = tokio::time::timeout(Duration::from_millis(500), rx2.recv())
        .await
        .expect("Timed out waiting for rx2")
        .expect("rx2 receive failed");
    assert_eq!(msg2.asset_id, "asset-nvda");
    assert_eq!(msg2.price_scaled, 120_000_000);
}

#[tokio::test]
async fn test_market_data_store_integer_scaling_precision() {
    // 1. Pyth expo -8 (common for USD equities)
    // raw = 185_5000_0000 ($185.50) -> 185_500_000 micro-USD
    assert_eq!(calculate_scaled_int(185_5000_0000, -8).unwrap(), 185_500_000);

    // 2. Pyth expo -6: 1:1 match with micro-USD
    assert_eq!(calculate_scaled_int(1_000_000, -6).unwrap(), 1_000_000);

    // 3. Pyth expo -5: scale up by 10
    // 100_000 ($1.00) -> 1_000_000 micro-USD
    assert_eq!(calculate_scaled_int(100_000, -5).unwrap(), 1_000_000);

    // 4. Pyth expo 0: scale up by 10^6
    assert_eq!(calculate_scaled_int(100, 0).unwrap(), 100_000_000);

    // 5. Half-up rounding test:
    // raw = 123456789 with expo -8 (1.23456789 USD) -> 1.234568 USD -> 1234568 micro-USD
    assert_eq!(calculate_scaled_int(123456789, -8).unwrap(), 1234568);

    // 6. Checked conversion rejects negative prices via MarketPriceUpdate::from_raw
    let neg_result = MarketPriceUpdate::from_raw(
        "asset-neg",
        "NEG",
        "MINT_NEG",
        "0xFEEDNEG",
        -500,
        -8,
        10,
        1700000000,
        30,
        None,
    );
    assert!(matches!(neg_result, Err(MarketDataError::InvalidPrice(_))));
}

#[tokio::test]
async fn test_market_data_store_freshness_metadata() {
    let now = Utc::now().timestamp();

    // Fresh record: published 5 seconds ago
    let fresh_update = MarketPriceUpdate {
        asset_id: "asset-fresh".to_string(),
        symbol: "FRESH".to_string(),
        mint_address: "MINT_FRESH".to_string(),
        pyth_feed_id: "0xFEEDFRESH".to_string(),
        price_scaled: 100_000_000,
        raw_price: 100_0000_0000,
        expo: -8,
        conf_scaled: 10_000,
        conf_raw: 10_0000,
        conf_bps: 1,
        price_usd: 100.0,
        conf_usd: 0.01,
        publish_time: now - 5,
        received_at: now,
        staleness_age_secs: 5,
        is_stale: false,
        slot: None,
    };

    assert_eq!(fresh_update.staleness_age_secs, 5);
    assert!(!fresh_update.is_stale);
    assert!(fresh_update.is_fresh(30));
    assert_eq!(fresh_update.freshness_status(30), PriceFreshness::Fresh);

    // Stale record: published 45 seconds ago with 30s threshold
    let stale_update = MarketPriceUpdate {
        publish_time: now - 45,
        staleness_age_secs: 45,
        is_stale: true,
        ..fresh_update.clone()
    };
    assert_eq!(stale_update.staleness_age_secs, 45);
    assert!(stale_update.is_stale);
    assert!(!stale_update.is_fresh(30));
    assert_eq!(stale_update.freshness_status(30), PriceFreshness::Stale);

    // Future skew: publish_time 10s ahead of local clock
    let future_update = MarketPriceUpdate {
        publish_time: now + 50,
        staleness_age_secs: -50,
        is_stale: true,
        ..fresh_update.clone()
    };
    assert_eq!(future_update.freshness_status(30), PriceFreshness::FutureSkew);
}

#[tokio::test]
async fn test_market_data_store_concurrent_reads_and_writes() {
    let store = Arc::new(MarketDataStore::new());
    let mut handles = Vec::new();

    // Seed 10 assets
    for i in 0..10 {
        let sub = make_test_subscription(
            &format!("asset-{i}"),
            &format!("SYM{i}"),
            &format!("MINT{i}"),
            &format!("0xFEED{i:04}"),
        );
        let feed = make_test_pyth_feed(&sub.pyth_feed_id, (100 + i as i64) * 1_0000_0000, 10_0000, -8, 1700000000);
        store.update(MarketPriceUpdate::from_pyth(&sub, &feed).unwrap()).await.unwrap();
    }

    // Spawn 15 concurrent reader tasks
    for r in 0..15 {
        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            for step in 0..100 {
                let pseudo_idx = (r * 13 + step * 7) % 10;
                let asset_id = format!("asset-{pseudo_idx}");
                let sym = format!("SYM{pseudo_idx}");
                let mint = format!("MINT{pseudo_idx}");

                let by_id = store_clone.get_by_asset_id(&asset_id).await;
                assert!(by_id.is_some());
                let by_sym = store_clone.get_by_symbol(&sym).await;
                assert!(by_sym.is_some());
                let by_mint = store_clone.get_by_mint(&mint).await;
                assert!(by_mint.is_some());
                tokio::task::yield_now().await;
            }
        });
        handles.push(handle);
    }

    // Spawn 10 concurrent writer tasks updating prices
    for w in 0..10 {
        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            let asset_id = format!("asset-{w}");
            let sub = make_test_subscription(
                &asset_id,
                &format!("SYM{w}"),
                &format!("MINT{w}"),
                &format!("0xFEED{w:04}"),
            );

            for step in 1..=50 {
                let feed = make_test_pyth_feed(
                    &sub.pyth_feed_id,
                    (200 + step) * 1_0000_0000,
                    10_0000,
                    -8,
                    1700000000 + step,
                );
                let update = MarketPriceUpdate::from_pyth(&sub, &feed).unwrap();
                store_clone.update(update).await.unwrap();
                tokio::task::yield_now().await;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task panicked during concurrent execution");
    }

    assert_eq!(store.len().await, 10);
}

#[tokio::test]
async fn test_phase6_acceptance_criteria_pyth_update_to_canonical_retrieval() {
    // ACCEPTANCE CRITERIA:
    // The application can receive a Pyth price update and retrieve the
    // latest validated market-data record by canonical asset identity.

    let store = MarketDataStore::new();

    // Canonical subscription setup from registry/database
    let canonical_asset_id = "018f9e2b-7c1a-7b3e-9f3a-123456789abc";
    let token_mint = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    let pyth_feed_id = "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43";
    let symbol = "SPY";

    let subscription = PythSubscription {
        asset_id: canonical_asset_id.to_string(),
        symbol: symbol.to_string(),
        mint_address: token_mint.to_string(),
        pyth_feed_id: pyth_feed_id.to_string(),
    };

    // 1. Simulate incoming Pyth Hermes SSE update
    let publish_time = Utc::now().timestamp();
    let pyth_incoming = ParsedPriceFeed {
        id: pyth_feed_id.to_string(),
        price: PythRawPrice {
            price: "52045000000".to_string(), // $520.45
            conf: "250000".to_string(),       // $0.0025 confidence
            expo: -8,
            publish_time,
        },
        ema_price: Some(PythRawPrice {
            price: "52040000000".to_string(),
            conf: "300000".to_string(),
            expo: -8,
            publish_time,
        }),
    };

    // 2. Validate and convert Pyth update to MarketPriceUpdate
    let market_update = MarketPriceUpdate::from_pyth(&subscription, &pyth_incoming)
        .expect("Valid Pyth feed should produce MarketPriceUpdate");

    // 3. Ingest into MarketDataStore
    store
        .update(market_update)
        .await
        .expect("Store update must succeed");

    // 4. Retrieve by canonical asset identity
    let retrieved = store
        .get_by_asset_id(canonical_asset_id)
        .await
        .expect("Must be retrievable by canonical asset_id");

    assert_eq!(retrieved.asset_id, canonical_asset_id);
    assert_eq!(retrieved.mint_address, token_mint);
    assert_eq!(retrieved.symbol, "SPY");
    assert_eq!(retrieved.price_scaled, 520_450_000); // 520.450000 micro-USD
    assert_eq!(retrieved.conf_scaled, 2_500);
    assert_eq!(retrieved.conf_bps, 0); // ~0.0048% -> rounds down to 0 bps
    assert_eq!(retrieved.publish_time, publish_time);
    assert!(!retrieved.is_stale);

    // 5. Verify negative/invalid price rejection
    let invalid_pyth = ParsedPriceFeed {
        id: pyth_feed_id.to_string(),
        price: PythRawPrice {
            price: "-100".to_string(), // Negative price invalid for equities
            conf: "10".to_string(),
            expo: -8,
            publish_time,
        },
        ema_price: None,
    };

    let result = MarketPriceUpdate::from_pyth(&subscription, &invalid_pyth);
    assert!(matches!(result, Err(MarketDataError::InvalidPrice(_))));
}
