//! Unit tests for PythSubscription and SubscriptionSet.

use equity_catalyst_pyth::{
    known_feeds, normalize_feed_id, PythSubscription, SubscriptionSet,
};

#[test]
fn test_pyth_subscription_creation_and_normalization() {
    let sub = PythSubscription::new(
        "backed:AAPLx",
        "aapl",
        "MintAAPLx111111111111111111111111111111111",
        format!("0x{}", known_feeds::AAPL_USD),
    );

    assert_eq!(sub.asset_id, "backed:AAPLx");
    assert_eq!(sub.symbol, "AAPL");
    assert_eq!(sub.mint_address, "MintAAPLx111111111111111111111111111111111");
    // Verifies 0x prefix is stripped and normalized to lowercase
    assert_eq!(sub.pyth_feed_id, known_feeds::AAPL_USD);
}

#[test]
fn test_empty_subscription_set() {
    let empty_set = SubscriptionSet::empty();
    assert!(empty_set.is_empty());
    assert_eq!(empty_set.len(), 0);
    assert_eq!(empty_set.subscriptions().len(), 0);
    assert_eq!(empty_set.feed_ids().len(), 0);
    assert_eq!(empty_set.symbols().len(), 0);
    assert!(!empty_set.contains_asset("backed:AAPLx"));
    assert!(!empty_set.contains_feed(known_feeds::AAPL_USD));
}

#[test]
fn test_single_asset_subscription_set() {
    let sub = PythSubscription::new(
        "backed:NVDAx",
        "NVDA",
        "MintNVDA111111111111111111111111111111111",
        known_feeds::NVDA_USD,
    );
    let set = SubscriptionSet::new(vec![sub.clone()]);

    assert!(!set.is_empty());
    assert_eq!(set.len(), 1);
    assert_eq!(set.feed_ids(), vec![known_feeds::NVDA_USD.to_string()]);
    assert_eq!(set.symbols(), vec!["NVDA".to_string()]);
    assert!(set.contains_asset("backed:NVDAx"));
    assert!(set.contains_feed(known_feeds::NVDA_USD));
    assert!(set.contains_feed(&format!("0x{}", known_feeds::NVDA_USD)));

    let found_by_asset = set.get_by_asset("backed:NVDAx").unwrap();
    assert_eq!(found_by_asset.symbol, "NVDA");

    let found_by_symbol = set.get_by_symbol("nvda").unwrap();
    assert_eq!(found_by_symbol.asset_id, "backed:NVDAx");

    let found_by_feed = set.get_by_feed(known_feeds::NVDA_USD).unwrap();
    assert_eq!(found_by_feed.symbol, "NVDA");
}

#[test]
fn test_multiple_assets_and_deterministic_ordering() {
    let sub_aapl = PythSubscription::new(
        "backed:AAPLx",
        "AAPL",
        "MintAAPL111111111111111111111111111111111",
        known_feeds::AAPL_USD,
    );
    let sub_tsla = PythSubscription::new(
        "backed:TSLAx",
        "TSLA",
        "MintTSLA111111111111111111111111111111111",
        known_feeds::TSLA_USD,
    );
    let sub_nvda = PythSubscription::new(
        "backed:NVDAx",
        "NVDA",
        "MintNVDA111111111111111111111111111111111",
        known_feeds::NVDA_USD,
    );

    // Set 1: AAPL, TSLA, NVDA
    let set1 = SubscriptionSet::new(vec![sub_aapl.clone(), sub_tsla.clone(), sub_nvda.clone()]);

    // Set 2: NVDA, AAPL, TSLA (different insertion order)
    let set2 = SubscriptionSet::new(vec![sub_nvda.clone(), sub_aapl.clone(), sub_tsla.clone()]);

    // Set 3: TSLA, NVDA, AAPL (yet another ordering)
    let set3 = SubscriptionSet::new(vec![sub_tsla.clone(), sub_nvda.clone(), sub_aapl.clone()]);

    // Deterministic equality regardless of order
    assert_eq!(set1, set2, "Sets with same subscriptions in different order must be equal");
    assert_eq!(set2, set3, "Sets with same subscriptions in different order must be equal");
    assert_eq!(set1, set3, "Sets with same subscriptions in different order must be equal");

    // Feed IDs must also be deterministic and sorted
    let feed_ids1 = set1.feed_ids();
    let feed_ids2 = set2.feed_ids();
    assert_eq!(feed_ids1, feed_ids2);
    assert_eq!(feed_ids1.len(), 3);
}

#[test]
fn test_feed_ids_deduplication() {
    let sub1 = PythSubscription::new(
        "asset:1",
        "A1",
        "Mint1",
        known_feeds::AAPL_USD,
    );
    let sub2 = PythSubscription::new(
        "asset:2",
        "A2",
        "Mint2",
        known_feeds::AAPL_USD, // same feed ID for different assets
    );

    let set = SubscriptionSet::new(vec![sub1, sub2]);
    let feed_ids = set.feed_ids();
    // feed_ids must deduplicate
    assert_eq!(feed_ids.len(), 1);
    assert_eq!(feed_ids[0], known_feeds::AAPL_USD);
}

#[test]
fn test_feed_id_normalization_helper() {
    assert_eq!(
        normalize_feed_id("0x49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175ec697df854ab"),
        "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175ec697df854ab"
    );
    assert_eq!(
        normalize_feed_id("  0X49F6B65CB1DE6B10EAF75E7C03CA029C306D0357E91B5311B175EC697DF854AB  "),
        "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175ec697df854ab"
    );
}
