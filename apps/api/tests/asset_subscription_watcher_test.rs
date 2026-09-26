//! Integration and dynamic lifecycle tests for Phase 3:
//! Pyth dynamic subscription models and AssetSubscriptionWatcher.

use equity_catalyst_api::{
    models::canonical_asset::CreateAssetRequest,
    repositories::{AssetMarketDataRepository, CanonicalAssetRepository},
    services::asset_subscription_watcher::{AssetSubscriptionWatcher, SubscriptionWatcherConfig},
};
use equity_catalyst_pyth::{known_feeds, PythSubscription, SubscriptionSet};
use equity_catalyst_shared::AssetApprovalStatus;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

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
const MSFT_FEED: &str = known_feeds::MSFT_USD;

/// Task 10: Empty subscription set test.
#[tokio::test]
async fn test_empty_subscription_set_watcher() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo);

    let config = SubscriptionWatcherConfig::new(Duration::from_secs(1));
    let watcher = AssetSubscriptionWatcher::new(market_repo, config);

    let current = watcher.current_subscription_set();
    assert!(current.is_empty());
    assert_eq!(current.len(), 0);
    assert_eq!(current.feed_ids().len(), 0);

    // Polling an empty repository should produce no change
    let changed = watcher.poll_once().await.expect("Poll should succeed");
    assert!(!changed, "Empty repo poll should not report changes");
}

/// Task 10: One asset subscription test.
#[tokio::test]
async fn test_single_asset_subscription_watcher() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    let req = sample_asset_request(
        "backed:AAPLx",
        "AAPL",
        "MintAAPL111111111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&req).await.unwrap();
    asset_repo.activate_asset("backed:AAPLx").await.unwrap();
    market_repo
        .create_mapping("backed:AAPLx", AAPL_FEED)
        .await
        .unwrap();

    let config = SubscriptionWatcherConfig::new(Duration::from_secs(1));
    let watcher = AssetSubscriptionWatcher::new(market_repo, config);
    let mut rx = watcher.subscribe();

    assert!(watcher.current_subscription_set().is_empty());

    // First poll detects the asset
    let changed = watcher.poll_once().await.unwrap();
    assert!(changed, "First poll must detect single active asset");

    let current = watcher.current_subscription_set();
    assert_eq!(current.len(), 1);
    assert_eq!(current.feed_ids(), vec![AAPL_FEED.to_string()]);
    assert_eq!(current.symbols(), vec!["AAPL".to_string()]);

    // Receiver should have received notification
    rx.changed().await.unwrap();
    assert_eq!(rx.borrow().len(), 1);

    // Second poll without DB changes must return false
    let changed_second = watcher.poll_once().await.unwrap();
    assert!(!changed_second, "Second poll must not notify if unchanged");
}

/// Task 10 & 4: Multiple assets and deterministic ordering (no false updates).
#[tokio::test]
async fn test_multiple_assets_and_deterministic_comparison() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    // Create AAPL and TSLA
    for (id, sym, mint, feed) in [
        ("backed:AAPLx", "AAPL", "MintAAPL", AAPL_FEED),
        ("backed:TSLAx", "TSLA", "MintTSLA", TSLA_FEED),
    ] {
        let req = sample_asset_request(id, sym, mint, AssetApprovalStatus::ShariahApproved);
        asset_repo.create_asset(&req).await.unwrap();
        asset_repo.activate_asset(id).await.unwrap();
        market_repo.create_mapping(id, feed).await.unwrap();
    }

    let config = SubscriptionWatcherConfig::new(Duration::from_secs(1));
    // Test new_initialized which eagerly loads initial state
    let watcher = AssetSubscriptionWatcher::new_initialized(market_repo, config)
        .await
        .unwrap();

    let initial = watcher.current_subscription_set();
    assert_eq!(initial.len(), 2);
    assert!(initial.contains_asset("backed:AAPLx"));
    assert!(initial.contains_asset("backed:TSLAx"));
    assert!(initial.contains_feed(AAPL_FEED));
    assert!(initial.contains_feed(TSLA_FEED));

    // Polling again must yield false (no false updates)
    for _ in 0..5 {
        let changed = watcher.poll_once().await.unwrap();
        assert!(!changed, "Repeated polls without changes must be false");
    }

    // Direct deterministic comparison verification
    let s1 = PythSubscription::new("backed:AAPLx", "AAPL", "MintAAPL", AAPL_FEED);
    let s2 = PythSubscription::new("backed:TSLAx", "TSLA", "MintTSLA", TSLA_FEED);

    let set_order_a = SubscriptionSet::new(vec![s1.clone(), s2.clone()]);
    let set_order_b = SubscriptionSet::new(vec![s2.clone(), s1.clone()]);
    assert_eq!(
        set_order_a, set_order_b,
        "Ordering differences must not produce inequality"
    );
}

/// ACCEPTANCE CRITERIA:
/// The system produces SubscriptionSet #1 [AAPL, TSLA],
/// then dynamically detects SubscriptionSet #2 [AAPL, TSLA, NVDA]
/// and notifies a listener without changing Rust source code.
#[tokio::test]
async fn test_acceptance_criteria_dynamic_subscription_update() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    // 1. Seed initial approved assets: AAPL and TSLA
    for (id, sym, mint, feed) in [
        ("backed:AAPLx", "AAPL", "MintAAPL", AAPL_FEED),
        ("backed:TSLAx", "TSLA", "MintTSLA", TSLA_FEED),
    ] {
        let req = sample_asset_request(id, sym, mint, AssetApprovalStatus::ShariahApproved);
        asset_repo.create_asset(&req).await.unwrap();
        asset_repo.activate_asset(id).await.unwrap();
        market_repo.create_mapping(id, feed).await.unwrap();
    }

    let config = SubscriptionWatcherConfig::new(Duration::from_millis(50));
    let watcher = AssetSubscriptionWatcher::new(market_repo.clone(), config);
    let mut listener_rx = watcher.subscribe();

    // First poll -> produces SubscriptionSet #1
    let changed_1 = watcher.poll_once().await.unwrap();
    assert!(changed_1, "Initial poll must notify listener of Set #1");

    listener_rx.changed().await.unwrap();
    let set_1 = listener_rx.borrow().clone();
    assert_eq!(
        set_1.len(),
        2,
        "SubscriptionSet #1 must contain AAPL and TSLA"
    );
    assert!(set_1.contains_symbol("AAPL") || set_1.contains_asset("backed:AAPLx"));
    assert!(set_1.contains_symbol("TSLA") || set_1.contains_asset("backed:TSLAx"));
    assert_eq!(set_1.feed_ids().len(), 2);

    // 2. Dynamically add NVDA asset and mapping in database (zero code changes)
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

    // Next poll -> detects SubscriptionSet #2
    let changed_2 = watcher.poll_once().await.unwrap();
    assert!(
        changed_2,
        "Poll must detect NVDA addition and notify listener of Set #2"
    );

    listener_rx.changed().await.unwrap();
    let set_2 = listener_rx.borrow().clone();
    assert_eq!(
        set_2.len(),
        3,
        "SubscriptionSet #2 must contain AAPL, TSLA, and NVDA"
    );
    assert!(set_2.contains_asset("backed:AAPLx"));
    assert!(set_2.contains_asset("backed:TSLAx"));
    assert!(set_2.contains_asset("backed:NVDAx"));
    assert_eq!(set_2.feed_ids().len(), 3);
    assert!(set_2.contains_feed(NVDA_FEED));
}

/// Task 10 & 12: Asset removal / deactivation makes asset disappear from SubscriptionSet.
#[tokio::test]
async fn test_asset_deactivation_removal() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    for (id, sym, mint, feed) in [
        ("backed:AAPLx", "AAPL", "MintAAPL", AAPL_FEED),
        ("backed:TSLAx", "TSLA", "MintTSLA", TSLA_FEED),
    ] {
        let req = sample_asset_request(id, sym, mint, AssetApprovalStatus::ShariahApproved);
        asset_repo.create_asset(&req).await.unwrap();
        asset_repo.activate_asset(id).await.unwrap();
        market_repo.create_mapping(id, feed).await.unwrap();
    }

    let config = SubscriptionWatcherConfig::new(Duration::from_millis(50));
    let watcher = AssetSubscriptionWatcher::new_initialized(market_repo.clone(), config)
        .await
        .unwrap();
    let mut rx = watcher.subscribe();

    assert_eq!(watcher.current_subscription_set().len(), 2);

    // Deactivate TSLA mapping in repository
    market_repo
        .deactivate_mapping("backed:TSLAx")
        .await
        .expect("Deactivation should succeed");

    // Watcher poll should detect removal
    let changed = watcher.poll_once().await.unwrap();
    assert!(changed, "Deactivation must trigger subscription set change");

    rx.changed().await.unwrap();
    let updated_set = rx.borrow().clone();
    assert_eq!(updated_set.len(), 1);
    assert!(updated_set.contains_asset("backed:AAPLx"));
    assert!(!updated_set.contains_asset("backed:TSLAx"));
    assert!(!updated_set.contains_feed(TSLA_FEED));
}

/// Task 10: Feed ID replacement dynamically updates the SubscriptionSet.
#[tokio::test]
async fn test_feed_id_replacement() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    let req = sample_asset_request(
        "backed:AAPLx",
        "AAPL",
        "MintAAPL",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&req).await.unwrap();
    asset_repo.activate_asset("backed:AAPLx").await.unwrap();
    market_repo
        .create_mapping("backed:AAPLx", AAPL_FEED)
        .await
        .unwrap();

    let config = SubscriptionWatcherConfig::new(Duration::from_millis(50));
    let watcher = AssetSubscriptionWatcher::new_initialized(market_repo.clone(), config)
        .await
        .unwrap();
    let mut rx = watcher.subscribe();

    assert_eq!(
        watcher.current_subscription_set().feed_ids(),
        vec![AAPL_FEED.to_string()]
    );

    // Replace feed ID for AAPL to MSFT_FEED
    market_repo
        .update_feed_mapping("backed:AAPLx", MSFT_FEED)
        .await
        .unwrap();

    let changed = watcher.poll_once().await.unwrap();
    assert!(changed, "Feed replacement must notify listener");

    rx.changed().await.unwrap();
    let updated_set = rx.borrow().clone();
    assert_eq!(updated_set.len(), 1);
    assert_eq!(updated_set.feed_ids(), vec![MSFT_FEED.to_string()]);
    assert_eq!(
        updated_set
            .get_by_asset("backed:AAPLx")
            .unwrap()
            .pyth_feed_id,
        MSFT_FEED
    );
}

/// Task 11 & 12: Newly activated approved asset included, deactivated asset removed.
#[tokio::test]
async fn test_asset_activation_and_deactivation_lifecycle() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    // Create asset with ShariahApproved status, but is_active = false
    let req = sample_asset_request(
        "backed:NVDAx",
        "NVDA",
        "MintNVDA",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&req).await.unwrap();
    // Inactive asset mapping cannot be created or is not active in get_active_mappings
    // Activate to create mapping
    asset_repo.activate_asset("backed:NVDAx").await.unwrap();
    market_repo
        .create_mapping("backed:NVDAx", NVDA_FEED)
        .await
        .unwrap();

    let config = SubscriptionWatcherConfig::new(Duration::from_millis(50));
    let watcher = AssetSubscriptionWatcher::new_initialized(market_repo.clone(), config)
        .await
        .unwrap();
    assert_eq!(watcher.current_subscription_set().len(), 1);

    // Deactivate the canonical asset itself (e.g. trading halt)
    asset_repo.deactivate_asset("backed:NVDAx").await.unwrap();

    let changed = watcher.poll_once().await.unwrap();
    assert!(
        changed,
        "Asset deactivation must remove it from SubscriptionSet"
    );
    assert_eq!(watcher.current_subscription_set().len(), 0);

    // Re-activate canonical asset via state machine transition (DEACTIVATED -> ACTIVE)
    asset_repo
        .update_approval_status("backed:NVDAx", AssetApprovalStatus::Active)
        .await
        .unwrap();

    let changed_reactivated = watcher.poll_once().await.unwrap();
    assert!(
        changed_reactivated,
        "Asset re-activation must re-add it to SubscriptionSet"
    );
    assert_eq!(watcher.current_subscription_set().len(), 1);
}

/// Task 9 & Background loop: Configurable interval and graceful shutdown.
#[tokio::test]
async fn test_background_watcher_loop_and_graceful_shutdown() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    // Configurable 20ms polling interval
    let config = SubscriptionWatcherConfig::new(Duration::from_millis(20));
    let watcher = Arc::new(AssetSubscriptionWatcher::new(market_repo.clone(), config));
    let mut rx = watcher.subscribe();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let handle = watcher.clone().start(shutdown_rx);

    // Initial state: empty
    assert!(rx.borrow().is_empty());

    // Add an approved asset
    let req = sample_asset_request(
        "backed:AAPLx",
        "AAPL",
        "MintAAPL",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&req).await.unwrap();
    asset_repo.activate_asset("backed:AAPLx").await.unwrap();
    market_repo
        .create_mapping("backed:AAPLx", AAPL_FEED)
        .await
        .unwrap();

    // The background watcher should automatically pick up the change within ~100ms
    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            rx.changed().await.unwrap();
            if rx.borrow().len() == 1 {
                break;
            }
        }
    })
    .await
    .expect("Background loop should detect change automatically");

    assert_eq!(rx.borrow().len(), 1);

    // Trigger graceful shutdown
    shutdown_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_millis(500), handle)
        .await
        .expect("Watcher handle must terminate gracefully")
        .expect("Task must not panic");
}

/// Phase P2 explicit repository methods and direct SubscriptionSet generation test.
#[tokio::test]
async fn test_phase_p2_repository_methods_and_subscription_set() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    // 1. Initially empty subscription set built from database
    let empty_set = market_repo
        .build_subscription_set()
        .await
        .expect("Building subscription set should succeed");
    assert!(empty_set.is_empty());
    assert_eq!(empty_set.len(), 0);

    // 2. Register assets with various lifecycle states
    // Asset 1: ShariahApproved & Activated
    let req1 = sample_asset_request(
        "backed:AAPLx",
        "AAPL",
        "MintAAPL111111111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&req1).await.unwrap();
    asset_repo.activate_asset("backed:AAPLx").await.unwrap();
    market_repo
        .create_mapping("backed:AAPLx", AAPL_FEED)
        .await
        .unwrap();

    // Asset 2: Pending (unapproved) - cannot have active mapping
    let req2 = sample_asset_request(
        "pnd:TSLAx",
        "TSLA",
        "MintTSLA111111111111111111111111111111111",
        AssetApprovalStatus::Pending,
    );
    asset_repo.create_asset(&req2).await.unwrap();
    assert!(market_repo
        .create_mapping("pnd:TSLAx", TSLA_FEED)
        .await
        .is_err());

    // Asset 3: ShariahApproved but not activated
    let req3 = sample_asset_request(
        "backed:NVDAx",
        "NVDA",
        "MintNVDA111111111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&req3).await.unwrap();
    market_repo
        .create_mapping("backed:NVDAx", NVDA_FEED)
        .await
        .unwrap();

    // 3. Test exact repository methods:
    // list_active_approved_assets
    let active_approved_assets = market_repo.list_active_approved_assets().await.unwrap();
    assert_eq!(active_approved_assets.len(), 1);
    assert_eq!(active_approved_assets[0].asset_id, "backed:AAPLx");

    let canonical_active_approved = asset_repo.list_active_approved_assets().await.unwrap();
    assert_eq!(canonical_active_approved.len(), 1);
    assert_eq!(canonical_active_approved[0].asset_id, "backed:AAPLx");

    // get_asset
    let asset_aapl = market_repo
        .get_asset("backed:AAPLx")
        .await
        .unwrap()
        .expect("Asset must be found");
    assert_eq!(asset_aapl.symbol, "AAPL");

    let canonical_asset_aapl = asset_repo
        .get_asset("backed:AAPLx")
        .await
        .unwrap()
        .expect("Asset must be found");
    assert_eq!(canonical_asset_aapl.symbol, "AAPL");

    // get_by_mint
    let asset_by_mint = market_repo
        .get_by_mint("MintAAPL111111111111111111111111111111111")
        .await
        .unwrap()
        .expect("Asset must be found by mint");
    assert_eq!(asset_by_mint.asset_id, "backed:AAPLx");

    let canonical_by_mint = asset_repo
        .get_by_mint("MintAAPL111111111111111111111111111111111")
        .await
        .unwrap()
        .expect("Asset must be found by mint");
    assert_eq!(canonical_by_mint.asset_id, "backed:AAPLx");

    // get_pyth_mapping
    let mapping_aapl = market_repo
        .get_pyth_mapping("backed:AAPLx")
        .await
        .unwrap()
        .expect("Mapping must be found");
    assert_eq!(mapping_aapl.pyth_feed_id, AAPL_FEED);

    // list_active_pyth_mappings (only active approved assets with active mapping)
    let active_mappings = market_repo.list_active_pyth_mappings().await.unwrap();
    assert_eq!(active_mappings.len(), 1);
    assert_eq!(active_mappings[0].asset_id, "backed:AAPLx");

    // 4. Build SubscriptionSet directly from database:
    let sub_set = market_repo.build_subscription_set().await.unwrap();
    assert_eq!(sub_set.len(), 1);
    assert_eq!(sub_set.feed_ids(), vec![AAPL_FEED.to_string()]);
    assert_eq!(sub_set.symbols(), vec!["AAPL".to_string()]);

    // 5. Activate Asset 3 (NVDA) in database -> automatically appears in SubscriptionSet
    asset_repo.activate_asset("backed:NVDAx").await.unwrap();
    let updated_sub_set = market_repo.build_subscription_set().await.unwrap();
    assert_eq!(updated_sub_set.len(), 2);
    assert!(updated_sub_set.contains_asset("backed:AAPLx"));
    assert!(updated_sub_set.contains_asset("backed:NVDAx"));

    // 6. Deactivate Asset 1 (AAPL) -> automatically removed from SubscriptionSet
    asset_repo.deactivate_asset("backed:AAPLx").await.unwrap();
    let final_sub_set = market_repo.build_subscription_set().await.unwrap();
    assert_eq!(final_sub_set.len(), 1);
    assert!(!final_sub_set.contains_asset("backed:AAPLx"));
    assert!(final_sub_set.contains_asset("backed:NVDAx"));
}
