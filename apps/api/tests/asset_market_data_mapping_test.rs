use equity_catalyst_api::{
    models::canonical_asset::CreateAssetRequest,
    repositories::{AssetMarketDataRepository, CanonicalAssetRepository},
};
use equity_catalyst_shared::AssetApprovalStatus;

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

const NVDA_FEED_ID: &str = "3155e714652285e6834d8ef0b3558163f4585c5b9679f222956cf57fb3645391";
const AAPL_FEED_ID: &str = "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175ec697df854ab";
const TSLA_FEED_ID: &str = "167776b6f68449c25605d8f6356499711202e0766e4a2d829910d54a165a22d7";
const MSFT_FEED_ID: &str = "4a985d8868f7004fdb85427d11129994c502b74fa6a2bc3053ba491a98059fa2";

#[tokio::test]
async fn test_valid_mapping_creation_and_retrieval() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    let req = sample_asset_request(
        "backed:NVDAx",
        "NVDA",
        "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&req).await.unwrap();
    asset_repo.activate_asset("backed:NVDAx").await.unwrap();

    // Create mapping with 0x prefix to verify normalization
    let raw_feed = format!("0x{}", NVDA_FEED_ID);
    let mapping = market_repo
        .create_mapping("backed:NVDAx", &raw_feed)
        .await
        .expect("Mapping creation should succeed");

    assert_eq!(mapping.asset_id, "backed:NVDAx");
    assert_eq!(mapping.symbol, "NVDA");
    assert_eq!(
        mapping.mint_address,
        "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh"
    );
    assert_eq!(mapping.pyth_feed_id, NVDA_FEED_ID);
    assert!(mapping.is_active);

    // Retrieve mapping for asset
    let fetched = market_repo
        .get_mapping_for_asset("backed:NVDAx")
        .await
        .expect("Query succeeded")
        .expect("Mapping found");

    assert_eq!(fetched.asset_id, "backed:NVDAx");
    assert_eq!(fetched.pyth_feed_id, NVDA_FEED_ID);
    assert!(fetched.is_active);
}

#[tokio::test]
async fn test_nonexistent_asset_rejection() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo);

    let res = market_repo
        .create_mapping("nonexistent:ASSET", NVDA_FEED_ID)
        .await;
    assert!(res.is_err(), "Must reject mapping for non-existent asset");
    assert!(res.unwrap_err().to_string().contains("does not exist"));
}

#[tokio::test]
async fn test_duplicate_mapping_rejection() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    let req1 = sample_asset_request(
        "backed:NVDAx",
        "NVDA",
        "MintNVDA11111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    let req2 = sample_asset_request(
        "backed:AAPLx",
        "AAPL",
        "MintAAPL11111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&req1).await.unwrap();
    asset_repo.create_asset(&req2).await.unwrap();

    market_repo
        .create_mapping("backed:NVDAx", NVDA_FEED_ID)
        .await
        .unwrap();

    // 1. Duplicate active mapping for the SAME asset must be rejected
    let dup_asset = market_repo
        .create_mapping("backed:NVDAx", AAPL_FEED_ID)
        .await;
    assert!(
        dup_asset.is_err(),
        "Must reject duplicate active mapping for the same asset"
    );
    assert!(dup_asset
        .unwrap_err()
        .to_string()
        .contains("already exists"));

    // 2. Duplicate active feed mapping to ANOTHER asset must be rejected
    let dup_feed = market_repo
        .create_mapping("backed:AAPLx", NVDA_FEED_ID)
        .await;
    assert!(
        dup_feed.is_err(),
        "Must reject mapping an active Pyth feed to multiple assets"
    );
    assert!(dup_feed.unwrap_err().to_string().contains("already mapped"));
}

#[tokio::test]
async fn test_inactive_mapping_and_inactive_asset_filtering() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    // Asset A: Active asset, Active mapping -> MUST BE INCLUDED
    let r1 = sample_asset_request(
        "asset:A",
        "AA",
        "MintA1111111111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&r1).await.unwrap();
    asset_repo.activate_asset("asset:A").await.unwrap();
    market_repo
        .create_mapping("asset:A", NVDA_FEED_ID)
        .await
        .unwrap();

    // Asset B: Active asset, Mapping deactivated -> MUST BE EXCLUDED
    let r2 = sample_asset_request(
        "asset:B",
        "BB",
        "MintB1111111111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&r2).await.unwrap();
    asset_repo.activate_asset("asset:B").await.unwrap();
    market_repo
        .create_mapping("asset:B", AAPL_FEED_ID)
        .await
        .unwrap();
    market_repo.deactivate_mapping("asset:B").await.unwrap();

    // Asset C: Mapping active, but Asset deactivated -> MUST BE EXCLUDED
    let r3 = sample_asset_request(
        "asset:C",
        "CC",
        "MintC1111111111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&r3).await.unwrap();
    asset_repo.activate_asset("asset:C").await.unwrap();
    market_repo
        .create_mapping("asset:C", TSLA_FEED_ID)
        .await
        .unwrap();
    asset_repo.deactivate_asset("asset:C").await.unwrap(); // Deactivate underlying asset

    // Asset D: ShariahApproved but NOT activated -> MUST BE EXCLUDED
    let r4 = sample_asset_request(
        "asset:D",
        "DD",
        "MintD1111111111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&r4).await.unwrap();
    market_repo
        .create_mapping("asset:D", MSFT_FEED_ID)
        .await
        .unwrap();

    // Answer the core acceptance criteria:
    // "For every currently active approved asset, which Pyth feed should provide its reference price?"
    let active_mappings = market_repo
        .get_active_mappings()
        .await
        .expect("Query failed");

    // Only Asset A must be returned
    assert_eq!(active_mappings.len(), 1);
    assert_eq!(active_mappings[0].asset_id, "asset:A");
    assert_eq!(active_mappings[0].symbol, "AA");
    assert_eq!(active_mappings[0].pyth_feed_id, NVDA_FEED_ID);

    let active_ids: Vec<&str> = active_mappings
        .iter()
        .map(|m| m.asset_id.as_str())
        .collect();
    assert!(!active_ids.contains(&"asset:B"));
    assert!(!active_ids.contains(&"asset:C"));
    assert!(!active_ids.contains(&"asset:D"));
}

#[tokio::test]
async fn test_feed_mapping_update() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    let req = sample_asset_request(
        "backed:NVDAx",
        "NVDA",
        "MintNVDA11111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&req).await.unwrap();
    asset_repo.activate_asset("backed:NVDAx").await.unwrap();

    market_repo
        .create_mapping("backed:NVDAx", NVDA_FEED_ID)
        .await
        .unwrap();

    // Update to new feed ID
    let updated = market_repo
        .update_feed_mapping("backed:NVDAx", TSLA_FEED_ID)
        .await
        .expect("Feed update should succeed");

    assert_eq!(updated.pyth_feed_id, TSLA_FEED_ID);

    let fetched = market_repo
        .get_mapping_for_asset("backed:NVDAx")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.pyth_feed_id, TSLA_FEED_ID);

    // Verify active mappings list reflects updated feed
    let active_list = market_repo.get_active_mappings().await.unwrap();
    assert_eq!(active_list.len(), 1);
    assert_eq!(active_list[0].pyth_feed_id, TSLA_FEED_ID);
}

#[tokio::test]
async fn test_approved_vs_non_approved_asset_behavior() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    // 1. Pending asset -> MUST FAIL
    let r_pending = sample_asset_request(
        "pnd:1",
        "PND",
        "MintP11111111111111111111111111111111",
        AssetApprovalStatus::Pending,
    );
    asset_repo.create_asset(&r_pending).await.unwrap();
    let res_pending = market_repo.create_mapping("pnd:1", NVDA_FEED_ID).await;
    assert!(res_pending.is_err(), "Must reject mapping to PENDING asset");
    assert!(res_pending
        .unwrap_err()
        .to_string()
        .contains("unapproved asset"));

    // 2. Validated asset -> MUST FAIL
    let r_val = sample_asset_request(
        "val:1",
        "VAL",
        "MintV11111111111111111111111111111111",
        AssetApprovalStatus::Validated,
    );
    asset_repo.create_asset(&r_val).await.unwrap();
    let res_val = market_repo.create_mapping("val:1", AAPL_FEED_ID).await;
    assert!(res_val.is_err(), "Must reject mapping to VALIDATED asset");
    assert!(res_val
        .unwrap_err()
        .to_string()
        .contains("unapproved asset"));

    // 3. ShariahApproved asset -> MUST SUCCEED
    let r_app = sample_asset_request(
        "app:1",
        "APP",
        "MintA11111111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&r_app).await.unwrap();
    let res_app = market_repo.create_mapping("app:1", TSLA_FEED_ID).await;
    assert!(
        res_app.is_ok(),
        "Must allow mapping to SHARIAH_APPROVED asset"
    );
}

#[tokio::test]
async fn test_empty_feed_id_rejection() {
    let asset_repo = CanonicalAssetRepository::new_in_memory();
    let market_repo = AssetMarketDataRepository::new_in_memory(asset_repo.clone());

    let req = sample_asset_request(
        "app:1",
        "APP",
        "MintA11111111111111111111111111111111",
        AssetApprovalStatus::ShariahApproved,
    );
    asset_repo.create_asset(&req).await.unwrap();

    assert!(market_repo.create_mapping("app:1", "").await.is_err());
    assert!(market_repo.create_mapping("app:1", "   ").await.is_err());
    assert!(market_repo.create_mapping("app:1", "0x").await.is_err());
}
