use chrono::Utc;
use equity_catalyst_pyth::{
    known_feeds, HermesLatestPriceResponse, NormalizedPrice, PythClient, PythFeedRegistry,
    PythRawPrice,
};

#[test]
fn test_hermes_json_deserialization() {
    let raw_json = r#"{
        "binary": {"encoding": "hex", "data": ["010203"]},
        "parsed": [
            {
                "id": "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d",
                "price": {
                    "price": "14523000000",
                    "conf": "15000000",
                    "expo": -8,
                    "publish_time": 1726000000
                },
                "ema_price": {
                    "price": "14510000000",
                    "conf": "16000000",
                    "expo": -8,
                    "publish_time": 1726000000
                }
            }
        ]
    }"#;

    let parsed: HermesLatestPriceResponse =
        serde_json::from_str(raw_json).expect("Failed to parse Hermes response");
    assert!(parsed.parsed.is_some());
    let feeds = parsed.parsed.unwrap();
    assert_eq!(feeds.len(), 1);
    assert_eq!(
        feeds[0].id,
        "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"
    );
    assert_eq!(feeds[0].price.price, "14523000000");
    assert_eq!(feeds[0].price.expo, -8);
}

#[test]
fn test_price_normalization_math() {
    let now = Utc::now().timestamp();
    let raw = PythRawPrice {
        price: "14523450000".to_string(), // 145.2345
        conf: "15000000".to_string(),     // 0.15
        expo: -8,
        publish_time: now,
    };

    let normalized = NormalizedPrice::from_raw("SOL", known_feeds::SOL_USD, &raw, 60)
        .expect("Normalization failed");

    assert_eq!(normalized.symbol, "SOL");
    assert!((normalized.price_usd - 145.2345).abs() < 1e-6);
    assert_eq!(normalized.price_scaled, 145_234_500);
    assert!((normalized.conf_usd - 0.15).abs() < 1e-6);
    assert!(!normalized.is_stale);
}

#[test]
fn test_staleness_detection() {
    let old_time = Utc::now().timestamp() - 300; // 5 minutes ago
    let raw = PythRawPrice {
        price: "100000000".to_string(),
        conf: "1000".to_string(),
        expo: -8,
        publish_time: old_time,
    };

    let normalized = NormalizedPrice::from_raw("USDC", known_feeds::USDC_USD, &raw, 60)
        .expect("Normalization succeeded");

    assert!(normalized.is_stale);
}

#[test]
fn test_feed_registry_lookups_and_custom_registration() {
    let registry = PythFeedRegistry::new();

    // Check defaults
    assert_eq!(registry.get_feed_id("SOL").unwrap(), known_feeds::SOL_USD);
    assert_eq!(registry.get_feed_id("USDC").unwrap(), known_feeds::USDC_USD);
    assert_eq!(registry.get_feed_id("AAPL").unwrap(), known_feeds::AAPL_USD);

    // Dynamic registration
    registry.register_feed(
        "GOOGL",
        "11223344556677889900aabbccddeeff11223344556677889900aabbccddeeff",
    );
    assert_eq!(
        registry.get_feed_id("GOOGL").unwrap(),
        "11223344556677889900aabbccddeeff11223344556677889900aabbccddeeff"
    );
    assert_eq!(
        registry
            .get_symbol("11223344556677889900aabbccddeeff11223344556677889900aabbccddeeff")
            .unwrap(),
        "GOOGL"
    );
}

#[tokio::test]
async fn test_mock_client_price_fetching() {
    let client = PythClient::new_mock();
    let now = Utc::now().timestamp();

    client.set_mock_price("SOL", "15000000000", "50000000", -8, now);

    let price = client
        .get_normalized_price_by_symbol("SOL", 60)
        .await
        .expect("Failed to get normalized price");

    assert_eq!(price.symbol, "SOL");
    assert_eq!(price.price_scaled, 150_000_000);
    assert!(!price.is_stale);
}
