use equity_catalyst_api::{
    models::canonical_asset::CreateAssetRequest, repositories::CanonicalAssetRepository,
};
use equity_catalyst_shared::AssetApprovalStatus;

fn create_sample_request(
    asset_id: &str,
    symbol: &str,
    mint_address: &str,
    status: Option<AssetApprovalStatus>,
) -> CreateAssetRequest {
    CreateAssetRequest {
        asset_id: asset_id.to_string(),
        symbol: symbol.to_string(),
        mint_address: mint_address.to_string(),
        legal_issuer: "Backed Finance AG".to_string(),
        custodian: "Maerki Baumann & Co. AG".to_string(),
        underlying_asset_identifier: format!("NASDAQ:{} (ISIN US0000000000)", symbol),
        is_active: false,
        approval_status: status,
        decimals: 8,
    }
}

#[tokio::test]
async fn test_create_and_retrieve_asset() {
    let repo = CanonicalAssetRepository::new_in_memory();

    let req = create_sample_request(
        "backed:NVDAx",
        "NVDA",
        "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh",
        None,
    );

    let created = repo
        .create_asset(&req)
        .await
        .expect("Failed to create asset");
    assert_eq!(created.asset_id, "backed:NVDAx");
    assert_eq!(created.symbol, "NVDA");
    assert_eq!(
        created.mint_address,
        "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh"
    );
    assert_eq!(created.legal_issuer, "Backed Finance AG");
    assert_eq!(created.custodian, "Maerki Baumann & Co. AG");
    assert_eq!(created.approval_status, AssetApprovalStatus::Pending);
    assert!(!created.is_active);
    assert_eq!(created.decimals, 8);

    // Retrieve by ID
    let by_id = repo
        .get_asset_by_id("backed:NVDAx")
        .await
        .expect("Query failed")
        .expect("Asset not found");
    assert_eq!(by_id.asset_id, created.asset_id);
    assert_eq!(by_id.mint_address, created.mint_address);

    // Retrieve by Mint
    let by_mint = repo
        .get_asset_by_mint("Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh")
        .await
        .expect("Query failed")
        .expect("Asset not found");
    assert_eq!(by_mint.asset_id, created.asset_id);

    // Non-existent lookups
    assert!(repo
        .get_asset_by_id("non_existent")
        .await
        .unwrap()
        .is_none());
    assert!(repo
        .get_asset_by_mint("non_existent_mint")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_duplicate_mint_rejection() {
    let repo = CanonicalAssetRepository::new_in_memory();
    let mint = "DuplicateMintAddress1111111111111111111111";

    let req1 = create_sample_request("backed:NVDAx", "NVDA", mint, None);
    repo.create_asset(&req1)
        .await
        .expect("First asset creation should succeed");

    // Attempt second asset with different asset_id and symbol but SAME mint address
    let req2 = create_sample_request("prestocks:NVDA", "NVDA-P", mint, None);
    let result = repo.create_asset(&req2).await;
    assert!(
        result.is_err(),
        "Duplicate mint address must be strictly rejected"
    );
    let err = result.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn test_duplicate_asset_id_rejection() {
    let repo = CanonicalAssetRepository::new_in_memory();

    let req1 = create_sample_request(
        "backed:NVDAx",
        "NVDA",
        "MintA11111111111111111111111111111111111",
        None,
    );
    repo.create_asset(&req1)
        .await
        .expect("First asset creation should succeed");

    let req2 = create_sample_request(
        "backed:NVDAx",
        "NVDA",
        "MintB11111111111111111111111111111111111",
        None,
    );
    let result = repo.create_asset(&req2).await;
    assert!(
        result.is_err(),
        "Duplicate asset ID must be strictly rejected"
    );
}

#[tokio::test]
async fn test_list_active_approved_assets_excludes_inactive() {
    let repo = CanonicalAssetRepository::new_in_memory();

    // 1. Pending asset
    let req_pending = create_sample_request(
        "asset:PENDING",
        "PND",
        "MintPending11111111111111111111111111111111",
        Some(AssetApprovalStatus::Pending),
    );
    repo.create_asset(&req_pending).await.unwrap();

    // 2. Validated asset
    let req_val = create_sample_request(
        "asset:VALIDATED",
        "VAL",
        "MintValidated11111111111111111111111111111111",
        Some(AssetApprovalStatus::Validated),
    );
    repo.create_asset(&req_val).await.unwrap();

    // 3. ShariahApproved asset (but not yet activated)
    let req_shariah = create_sample_request(
        "asset:SHARIAH_APPROVED",
        "SHR",
        "MintShariah1111111111111111111111111111111111",
        Some(AssetApprovalStatus::ShariahApproved),
    );
    repo.create_asset(&req_shariah).await.unwrap();

    // 4. ShariahApproved asset that gets activated
    let req_active = create_sample_request(
        "asset:ACTIVE",
        "ACT",
        "MintActive11111111111111111111111111111111111",
        Some(AssetApprovalStatus::ShariahApproved),
    );
    repo.create_asset(&req_active).await.unwrap();
    let activated = repo
        .activate_asset("asset:ACTIVE")
        .await
        .expect("Activation should succeed");
    assert!(activated.is_active);
    assert_eq!(activated.approval_status, AssetApprovalStatus::Active);

    // Query active approved assets (used for live market data subscriptions)
    let active_approved = repo
        .list_active_approved_assets()
        .await
        .expect("Query failed");

    // ONLY the activated asset must be returned
    assert_eq!(active_approved.len(), 1);
    assert_eq!(active_approved[0].asset_id, "asset:ACTIVE");
    assert_eq!(active_approved[0].symbol, "ACT");
    assert!(active_approved[0].is_active);

    // Verify all other assets were strictly excluded
    let active_ids: Vec<&str> = active_approved
        .iter()
        .map(|a| a.asset_id.as_str())
        .collect();
    assert!(!active_ids.contains(&"asset:PENDING"));
    assert!(!active_ids.contains(&"asset:VALIDATED"));
    assert!(!active_ids.contains(&"asset:SHARIAH_APPROVED"));
}

#[tokio::test]
async fn test_approval_status_filtering() {
    let repo = CanonicalAssetRepository::new_in_memory();

    let p1 = create_sample_request(
        "id:1",
        "S1",
        "Mint111111111111111111111111111111111111111",
        Some(AssetApprovalStatus::Pending),
    );
    let v1 = create_sample_request(
        "id:2",
        "S2",
        "Mint222222222222222222222222222222222222222",
        Some(AssetApprovalStatus::Validated),
    );
    let s1 = create_sample_request(
        "id:3",
        "S3",
        "Mint333333333333333333333333333333333333333",
        Some(AssetApprovalStatus::ShariahApproved),
    );

    repo.create_asset(&p1).await.unwrap();
    repo.create_asset(&v1).await.unwrap();
    repo.create_asset(&s1).await.unwrap();

    let pending_list = repo
        .list_by_status(AssetApprovalStatus::Pending)
        .await
        .unwrap();
    assert_eq!(pending_list.len(), 1);
    assert_eq!(pending_list[0].asset_id, "id:1");

    let val_list = repo
        .list_by_status(AssetApprovalStatus::Validated)
        .await
        .unwrap();
    assert_eq!(val_list.len(), 1);
    assert_eq!(val_list[0].asset_id, "id:2");

    let shariah_list = repo
        .list_by_status(AssetApprovalStatus::ShariahApproved)
        .await
        .unwrap();
    assert_eq!(shariah_list.len(), 1);
    assert_eq!(shariah_list[0].asset_id, "id:3");

    let active_list = repo
        .list_by_status(AssetApprovalStatus::Active)
        .await
        .unwrap();
    assert!(active_list.is_empty());
}

#[tokio::test]
async fn test_activation_and_deactivation_lifecycle() {
    let repo = CanonicalAssetRepository::new_in_memory();

    let req = create_sample_request(
        "backed:AAPLx",
        "AAPL",
        "XsbEhLAtcf6HdfpFZ5xEMdqW8nfAvcsP5bdudRLJzJp",
        Some(AssetApprovalStatus::Pending),
    );
    repo.create_asset(&req).await.unwrap();

    // 1. Activation directly from PENDING must fail closed
    let act_err = repo.activate_asset("backed:AAPLx").await;
    assert!(act_err.is_err(), "Must reject activation of PENDING asset");
    assert!(act_err
        .unwrap_err()
        .to_string()
        .contains("must be 'SHARIAH_APPROVED'"));

    // 2. Transition PENDING -> VALIDATED
    let validated = repo
        .update_approval_status("backed:AAPLx", AssetApprovalStatus::Validated)
        .await
        .expect("Transition to VALIDATED should succeed");
    assert_eq!(validated.approval_status, AssetApprovalStatus::Validated);
    assert!(!validated.is_active);

    // 3. Activation from VALIDATED must still fail closed
    assert!(repo.activate_asset("backed:AAPLx").await.is_err());

    // 4. Transition VALIDATED -> SHARIAH_APPROVED
    let shariah = repo
        .update_approval_status("backed:AAPLx", AssetApprovalStatus::ShariahApproved)
        .await
        .expect("Transition to SHARIAH_APPROVED should succeed");
    assert_eq!(
        shariah.approval_status,
        AssetApprovalStatus::ShariahApproved
    );
    assert!(!shariah.is_active);

    // 5. Activation from SHARIAH_APPROVED succeeds
    let active = repo
        .activate_asset("backed:AAPLx")
        .await
        .expect("Activation of SHARIAH_APPROVED asset must succeed");
    assert_eq!(active.approval_status, AssetApprovalStatus::Active);
    assert!(active.is_active);

    let active_list = repo.list_active_approved_assets().await.unwrap();
    assert_eq!(active_list.len(), 1);

    // 6. Deactivation immediately halts and removes asset from active pool
    let deactivated = repo
        .deactivate_asset("backed:AAPLx")
        .await
        .expect("Deactivation should succeed");
    assert_eq!(
        deactivated.approval_status,
        AssetApprovalStatus::Deactivated
    );
    assert!(!deactivated.is_active);

    let active_list_after = repo.list_active_approved_assets().await.unwrap();
    assert!(
        active_list_after.is_empty(),
        "Deactivated asset must NOT be in active list"
    );
}

#[tokio::test]
async fn test_symbol_is_not_canonical_identity() {
    let repo = CanonicalAssetRepository::new_in_memory();

    // Two distinct assets with the SAME ticker symbol "NVDA", but distinct asset_id and mint_address
    let req1 = create_sample_request(
        "backed:NVDAx",
        "NVDA",
        "MintBackedNVDA111111111111111111111111111111",
        Some(AssetApprovalStatus::ShariahApproved),
    );
    let req2 = create_sample_request(
        "tessera:NVDA",
        "NVDA",
        "MintTesseraNVDA11111111111111111111111111111",
        Some(AssetApprovalStatus::ShariahApproved),
    );

    let a1 = repo
        .create_asset(&req1)
        .await
        .expect("Creation 1 succeeded");
    let a2 = repo
        .create_asset(&req2)
        .await
        .expect("Creation 2 succeeded");

    assert_ne!(a1.asset_id, a2.asset_id);
    assert_ne!(a1.mint_address, a2.mint_address);
    assert_eq!(a1.symbol, a2.symbol);

    assert_eq!(
        repo.get_asset_by_id("backed:NVDAx")
            .await
            .unwrap()
            .unwrap()
            .mint_address,
        a1.mint_address
    );
    assert_eq!(
        repo.get_asset_by_id("tessera:NVDA")
            .await
            .unwrap()
            .unwrap()
            .mint_address,
        a2.mint_address
    );
}
