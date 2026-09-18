use deadpool_postgres::Pool;
use equity_catalyst_api::{
    config::Config, create_db_pool, error::ApiError, services::SolanaService,
};
use equity_catalyst_solana::anchor_client::*;
use solana_sdk::signature::{Keypair, Signer};

fn setup_test_pool() -> Pool {
    let config = Config::from_env();
    create_db_pool(&config.database_url).expect("Failed to connect to test postgres")
}

#[tokio::test]
async fn test_solana_service_read_only_default_and_transition() {
    let service = SolanaService::new(
        "https://api.devnet.solana.com",
        "wss://api.devnet.solana.com",
        None,
        None,
    );

    // 1. Verify read-only is strictly enforced by default
    assert!(service.is_read_only().await);

    // 2. Attempting transaction submission must be rejected
    let dummy_user = Keypair::new();
    let dummy_vault = Keypair::new().pubkey();
    let dummy_ata = Keypair::new().pubkey();
    let dummy_mint = Keypair::new().pubkey();

    let res = service
        .deposit(
            &dummy_user,
            &dummy_vault,
            &dummy_ata,
            &dummy_ata,
            &dummy_mint,
            1000,
        )
        .await;

    match res {
        Err(ApiError::BadRequest(msg)) => {
            assert!(msg.contains("read-only mode"));
        }
        other => panic!("Expected BadRequest read-only rejection, got {:?}", other),
    }

    // 3. Transition to enabled write mode
    service.enable_transaction_submission().await;
    assert!(!service.is_read_only().await);
}

#[tokio::test]
async fn test_solana_service_account_reads_and_validation() {
    let service = SolanaService::new(
        "https://api.devnet.solana.com",
        "wss://api.devnet.solana.com",
        None,
        None,
    );

    // Invalid base58 address should return BadRequest
    let err_res = service.read_vault_state("not_a_base58_address").await;
    assert!(matches!(err_res, Err(ApiError::BadRequest(_))));

    // Non-existent address on Devnet should return Ok(None)
    let random_vault = Keypair::new().pubkey().to_string();
    let none_res = service.read_vault_state(&random_vault).await;
    assert!(matches!(none_res, Ok(None)));

    // Read user shares for non-existent vault
    let random_user = Keypair::new().pubkey().to_string();
    let shares_res = service.read_user_shares(&random_vault, &random_user).await;
    assert!(matches!(shares_res, Ok(None)));
}

#[tokio::test]
async fn test_solana_service_db_sync_with_missing_onchain_vault() {
    let pool = setup_test_pool();
    let service = SolanaService::new(
        "https://api.devnet.solana.com",
        "wss://api.devnet.solana.com",
        None,
        None,
    );

    let random_vault = Keypair::new().pubkey().to_string();
    let sync_res = service.sync_vault_state_to_db(&random_vault, &pool).await;
    assert!(matches!(sync_res, Ok(None)));
}

#[tokio::test]
async fn test_anchor_instruction_builders_via_service() {
    let service = SolanaService::new(
        "https://api.devnet.solana.com",
        "wss://api.devnet.solana.com",
        None,
        None,
    );

    let authority = Keypair::new();
    let asset_mint = Keypair::new().pubkey();

    // 1. Initialize vault instruction
    let (init_ix, vault_pda, policy_pda) = service
        .anchor_client()
        .build_initialize_vault_ix(
            &authority.pubkey(),
            &asset_mint,
            "Service Alpha",
            "S-ALP",
            7500,
            2500,
        )
        .expect("Failed to build initialize_vault ix");

    assert_eq!(&init_ix.data[..8], &INITIALIZE_VAULT_DISCRIMINATOR);
    assert_eq!(init_ix.accounts[1].pubkey, vault_pda);
    assert_eq!(init_ix.accounts[2].pubkey, policy_pda);

    // 2. Deposit instruction
    let user = Keypair::new();
    let user_ata = Keypair::new().pubkey();
    let vault_ata = Keypair::new().pubkey();
    let (deposit_ix, shares_pda) = service
        .anchor_client()
        .build_deposit_ix(
            &user.pubkey(),
            &vault_pda,
            &user_ata,
            &vault_ata,
            &asset_mint,
            5_000_000,
        )
        .expect("Failed to build deposit ix");

    assert_eq!(&deposit_ix.data[..8], &DEPOSIT_DISCRIMINATOR);
    assert_eq!(deposit_ix.accounts[2].pubkey, shares_pda);

    // 3. Withdraw instruction
    let (withdraw_ix, _) = service
        .anchor_client()
        .build_withdraw_ix(
            &user.pubkey(),
            &vault_pda,
            &user_ata,
            &vault_ata,
            &asset_mint,
            2_500_000,
        )
        .expect("Failed to build withdraw ix");

    assert_eq!(&withdraw_ix.data[..8], &WITHDRAW_DISCRIMINATOR);

    // 4. Update policy instruction
    let (update_ix, p_pda) = service
        .anchor_client()
        .build_update_policy_ix(
            &authority.pubkey(),
            &vault_pda,
            8000,
            3000,
            600,
            2000,
            250,
            true,
        )
        .expect("Failed to build update_policy ix");

    assert_eq!(&update_ix.data[..8], &UPDATE_POLICY_DISCRIMINATOR);
    assert_eq!(p_pda, policy_pda);

    // 5. Emergency exit instruction
    let exit_ix = service
        .anchor_client()
        .build_emergency_exit_ix(&authority.pubkey(), &vault_pda, true)
        .expect("Failed to build emergency_exit ix");

    assert_eq!(&exit_ix.data[..8], &EMERGENCY_EXIT_DISCRIMINATOR);

    // 6. Execute action instruction
    let (compliance_pda, _) = equity_catalyst_solana::accounts::find_compliance_pda(&asset_mint, service.program_id());
    let (action_ix, _) = service
        .anchor_client()
        .build_execute_action_ix(
            &authority.pubkey(),
            &vault_pda,
            1,
            1, // Spot Swap
            &asset_mint,
            &asset_mint,
            &compliance_pda,
            1_000_000,
            990_000,
        )
        .expect("Failed to build execute_action ix");
    assert_eq!(action_ix.accounts[0].pubkey, authority.pubkey());
    assert_eq!(action_ix.accounts[5].pubkey, compliance_pda);

    // 7. Borrow instruction
    let borrower = Keypair::new();
    let (borrow_ix, _) = service
        .anchor_client()
        .build_borrow_ix(&borrower.pubkey(), &vault_pda, 500_000)
        .expect("Failed to build borrow ix");
    assert_eq!(borrow_ix.accounts[0].pubkey, borrower.pubkey());

    // 8. Repay instruction
    let (repay_ix, _) = service
        .anchor_client()
        .build_repay_ix(&borrower.pubkey(), &vault_pda, 500_000)
        .expect("Failed to build repay ix");
    assert_eq!(repay_ix.accounts[0].pubkey, borrower.pubkey());
}
