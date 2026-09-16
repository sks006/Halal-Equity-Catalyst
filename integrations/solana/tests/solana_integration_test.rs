use borsh::BorshSerialize;
use equity_catalyst_solana::{
    accounts::*, anchor_client::*, rpc::SolanaRpcClient, AnchorClient, SolanaError,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::sync::Arc;

#[test]
fn test_anchor_discriminators_match_idl() {
    assert_eq!(
        VAULT_ACCOUNT_DISCRIMINATOR,
        compute_account_discriminator("Vault")
    );
    assert_eq!(
        POLICY_ACCOUNT_DISCRIMINATOR,
        compute_account_discriminator("Policy")
    );
    assert_eq!(
        USER_SHARES_ACCOUNT_DISCRIMINATOR,
        compute_account_discriminator("UserShares")
    );
    assert_eq!(
        POSITION_ACCOUNT_DISCRIMINATOR,
        compute_account_discriminator("Position")
    );
    assert_eq!(
        LOAN_ACCOUNT_DISCRIMINATOR,
        compute_account_discriminator("Loan")
    );
    assert_eq!(
        EXECUTION_ACCOUNT_DISCRIMINATOR,
        compute_account_discriminator("Execution")
    );

    assert_eq!(
        INITIALIZE_VAULT_DISCRIMINATOR,
        compute_instruction_discriminator("initialize_vault")
    );
    assert_eq!(
        DEPOSIT_DISCRIMINATOR,
        compute_instruction_discriminator("deposit")
    );
    assert_eq!(
        WITHDRAW_DISCRIMINATOR,
        compute_instruction_discriminator("withdraw")
    );
    assert_eq!(
        UPDATE_POLICY_DISCRIMINATOR,
        compute_instruction_discriminator("update_policy")
    );
    assert_eq!(
        EMERGENCY_EXIT_DISCRIMINATOR,
        compute_instruction_discriminator("emergency_exit")
    );
}

#[test]
fn test_pda_derivations() {
    let program = program_id();
    let authority = Keypair::new().pubkey();
    let name = "Catalyst Alpha";

    let (vault_pda, bump) = find_vault_pda(&authority, name, &program);
    assert_ne!(vault_pda, Pubkey::default());
    let _ = bump;

    let (policy_pda, p_bump) = find_policy_pda(&vault_pda, &program);
    assert_ne!(policy_pda, Pubkey::default());
    let _ = p_bump;

    let user = Keypair::new().pubkey();
    let (shares_pda, s_bump) = find_user_shares_pda(&vault_pda, &user, &program);
    assert_ne!(shares_pda, Pubkey::default());
    let _ = s_bump;

    let (loan_pda, _) = find_loan_pda(&vault_pda, &user, &program);
    assert_ne!(loan_pda, Pubkey::default());

    let mint = Keypair::new().pubkey();
    let (pos_pda, _) = find_position_pda(&vault_pda, &mint, &program);
    assert_ne!(pos_pda, Pubkey::default());

    let (exec_pda, _) = find_execution_pda(&vault_pda, 42, &program);
    assert_ne!(exec_pda, Pubkey::default());
}

#[test]
fn test_vault_account_deserialization() {
    let authority = Keypair::new().pubkey();
    let asset_mint = Keypair::new().pubkey();
    let policy = Keypair::new().pubkey();

    let vault = VaultAccount {
        authority,
        asset_mint,
        policy,
        name: "Catalyst Alpha".to_string(),
        symbol: "CAT-A".to_string(),
        total_deposits: 10_000_000,
        total_shares: 10_000_000,
        is_paused: false,
        bump: 254,
        created_at: 1700000000,
        updated_at: 1700000100,
    };

    let mut data = Vec::new();
    data.extend_from_slice(&VAULT_ACCOUNT_DISCRIMINATOR);
    vault.serialize(&mut data).unwrap();

    let parsed = parse_vault(&data).expect("Failed to parse valid vault account");
    assert_eq!(parsed, vault);

    // Test invalid discriminator
    let mut bad_data = data.clone();
    bad_data[0] ^= 0xFF;
    assert!(parse_vault(&bad_data).is_err());

    // Test truncated data
    assert!(parse_vault(&data[..7]).is_err());
}

#[test]
fn test_policy_account_deserialization() {
    let vault = Keypair::new().pubkey();
    let authority = Keypair::new().pubkey();

    let policy = PolicyAccount {
        vault,
        authority,
        max_ltv_bps: 7500,
        max_position_bps: 2500,
        stop_loss_bps: 500,
        take_profit_bps: 1500,
        rebalance_threshold_bps: 200,
        is_active: true,
        bump: 253,
        updated_at: 1700000200,
    };

    let mut data = Vec::new();
    data.extend_from_slice(&POLICY_ACCOUNT_DISCRIMINATOR);
    policy.serialize(&mut data).unwrap();

    let parsed = parse_policy(&data).expect("Failed to parse valid policy account");
    assert_eq!(parsed, policy);
}

#[test]
fn test_user_shares_account_deserialization() {
    let vault = Keypair::new().pubkey();
    let user = Keypair::new().pubkey();

    let shares = UserSharesAccount {
        vault,
        user,
        shares: 500_000,
        bump: 255,
        updated_at: 1700000300,
    };

    let mut data = Vec::new();
    data.extend_from_slice(&USER_SHARES_ACCOUNT_DISCRIMINATOR);
    shares.serialize(&mut data).unwrap();

    let parsed = parse_user_shares(&data).expect("Failed to parse valid user shares");
    assert_eq!(parsed, shares);
}

#[test]
fn test_spl_token_account_decoding() {
    let mint = Keypair::new().pubkey();
    let owner = Keypair::new().pubkey();
    let amount: u64 = 7_500_000;

    let mut data = vec![0u8; 165];
    data[0..32].copy_from_slice(mint.as_ref());
    data[32..64].copy_from_slice(owner.as_ref());
    data[64..72].copy_from_slice(&amount.to_le_bytes());
    data[108] = 1; // Initialized

    let parsed = parse_spl_token(&data).expect("Failed to decode standard SPL token account");
    assert_eq!(parsed.mint, mint);
    assert_eq!(parsed.owner, owner);
    assert_eq!(parsed.amount, amount);
    assert!(parsed.is_initialized);
    assert!(!parsed.is_frozen);
}

#[test]
fn test_instruction_builders_layout() {
    let rpc = Arc::new(SolanaRpcClient::new("https://api.devnet.solana.com"));
    let client = AnchorClient::new(rpc);

    let authority = Keypair::new().pubkey();
    let asset_mint = Keypair::new().pubkey();

    // 1. Initialize vault
    let (init_ix, v_pda, p_pda) = client
        .build_initialize_vault_ix(&authority, &asset_mint, "Alpha", "ALP", 7500, 2500)
        .unwrap();
    assert_eq!(&init_ix.data[..8], &INITIALIZE_VAULT_DISCRIMINATOR);
    assert_eq!(init_ix.accounts.len(), 5);
    assert_eq!(init_ix.accounts[1].pubkey, v_pda);
    assert_eq!(init_ix.accounts[2].pubkey, p_pda);

    // 2. Update policy
    let (policy_ix, _) = client
        .build_update_policy_ix(&authority, &v_pda, 8000, 3000, 600, 2000, 250, true)
        .unwrap();
    assert_eq!(&policy_ix.data[..8], &UPDATE_POLICY_DISCRIMINATOR);
    assert_eq!(policy_ix.accounts.len(), 3);

    // 3. Deposit
    let user = Keypair::new().pubkey();
    let user_ata = Keypair::new().pubkey();
    let vault_ata = Keypair::new().pubkey();
    let (dep_ix, shares_pda) = client
        .build_deposit_ix(&user, &v_pda, &user_ata, &vault_ata, &asset_mint, 1_000_000)
        .unwrap();
    assert_eq!(&dep_ix.data[..8], &DEPOSIT_DISCRIMINATOR);
    assert_eq!(dep_ix.accounts[2].pubkey, shares_pda);

    // 4. Withdraw
    let (wdr_ix, _) = client
        .build_withdraw_ix(&user, &v_pda, &user_ata, &vault_ata, &asset_mint, 500_000)
        .unwrap();
    assert_eq!(&wdr_ix.data[..8], &WITHDRAW_DISCRIMINATOR);

    // 5. Emergency exit
    let emg_ix = client
        .build_emergency_exit_ix(&authority, &v_pda, true)
        .unwrap();
    assert_eq!(&emg_ix.data[..8], &EMERGENCY_EXIT_DISCRIMINATOR);
    assert_eq!(emg_ix.accounts.len(), 2);
}

#[tokio::test]
async fn test_read_only_mode_guard() {
    let rpc = Arc::new(SolanaRpcClient::new("https://api.devnet.solana.com"));
    let mut client = AnchorClient::new(rpc);

    // Verify default is read-only
    assert!(!client.is_transaction_submission_enabled());

    let dummy_ix = client
        .build_emergency_exit_ix(&Keypair::new().pubkey(), &Keypair::new().pubkey(), true)
        .unwrap();
    let payer = Keypair::new();

    // Submission MUST be rejected when read-only
    let result = client
        .send_and_confirm_transaction(&[dummy_ix.clone()], &payer.pubkey(), &[&payer])
        .await;

    match result {
        Err(SolanaError::TransactionsDisabled) => {} // Expected
        other => panic!("Expected TransactionsDisabled error, got {:?}", other),
    }

    // Now enable transactions
    client.enable_transaction_submission();
    assert!(client.is_transaction_submission_enabled());
}

#[test]
fn test_verified_deployed_program_ids() {
    use std::str::FromStr;

    // 1. Anchor Equity Vault Program
    let vault_pid = Pubkey::from_str(PROGRAM_ID_STR).expect("Valid Equity Vault program ID");
    assert_eq!(
        vault_pid.to_string(),
        "8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH"
    );

    // 2. Official Meteora Dynamic Bonding Curve (DBC) Program
    let dbc_pid = Pubkey::from_str(METEORA_DBC_PROGRAM_ID).expect("Valid Meteora DBC program ID");
    assert_eq!(
        dbc_pid.to_string(),
        "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"
    );

    // 3. Jupiter v6 Swap Program
    let jup_pid = Pubkey::from_str(JUPITER_V6_PROGRAM_ID).expect("Valid Jupiter program ID");
    assert_eq!(
        jup_pid.to_string(),
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"
    );

    // 4. SPL Token Program
    let spl_token = Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).expect("Valid SPL Token program ID");
    assert_eq!(
        spl_token.to_string(),
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    );

    // 5. SPL Associated Token Account Program
    let spl_ata =
        Pubkey::from_str(SPL_ASSOCIATED_TOKEN_PROGRAM_ID).expect("Valid SPL ATA program ID");
    assert_eq!(
        spl_ata.to_string(),
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
    );
}

#[test]
fn test_rpc_client_builder_and_endpoints() {
    use std::time::Duration;

    let primary = "https://api.mainnet-beta.solana.com";
    let fb1 = "https://rpc.ankr.com/solana";
    let fb2 = "https://solana-mainnet.g.alchemy.com/v2/demo";

    let client = SolanaRpcClient::new(primary)
        .with_fallback(fb1)
        .with_fallback(fb2)
        // Test deduplication
        .with_fallback(primary)
        .with_fallback(fb1)
        .with_timeout(Duration::from_secs(12))
        .with_max_retries(3)
        .with_commitment("finalized");

    assert_eq!(client.rpc_url(), primary);
    assert_eq!(client.fallback_urls(), &[fb1.to_string(), fb2.to_string()]);
    assert_eq!(
        client.all_endpoints(),
        vec![primary.to_string(), fb1.to_string(), fb2.to_string()]
    );
    assert_eq!(client.timeout(), Duration::from_secs(12));
    assert_eq!(client.max_retries(), 3);
    assert_eq!(client.commitment(), "finalized");
}

#[tokio::test]
async fn test_rpc_client_failover_on_503() {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // 1. Failing primary server responding with HTTP 503
    let listener_fail = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port_fail = listener_fail.local_addr().unwrap().port();
    let fail_url = format!("http://127.0.0.1:{}", port_fail);

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener_fail.accept().await {
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let response = "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n";
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    // 2. Healthy fallback server responding with valid getBalance JSON-RPC
    let listener_ok = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port_ok = listener_ok.local_addr().unwrap().port();
    let ok_url = format!("http://127.0.0.1:{}", port_ok);

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener_ok.accept().await {
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let body = r#"{"jsonrpc":"2.0","result":{"value":75000000},"id":1}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    // 3. Client configured with failing primary and healthy fallback
    let client = SolanaRpcClient::new(fail_url)
        .with_fallback(ok_url)
        .with_timeout(Duration::from_secs(2))
        .with_max_retries(1);

    let dummy_key = Keypair::new().pubkey();
    let balance = client.get_balance(&dummy_key).await;

    assert!(
        balance.is_ok(),
        "Should failover to healthy endpoint successfully: {:?}",
        balance.err()
    );
    assert_eq!(balance.unwrap(), 75_000_000);
}

#[tokio::test]
async fn test_rpc_client_failover_on_429() {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // Failing primary server responding with HTTP 429 Too Many Requests
    let listener_fail = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port_fail = listener_fail.local_addr().unwrap().port();
    let fail_url = format!("http://127.0.0.1:{}", port_fail);

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener_fail.accept().await {
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let response = "HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\n\r\n";
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    // Healthy fallback server
    let listener_ok = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port_ok = listener_ok.local_addr().unwrap().port();
    let ok_url = format!("http://127.0.0.1:{}", port_ok);

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener_ok.accept().await {
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let body = r#"{"jsonrpc":"2.0","result":{"value":123456},"id":1}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    let client = SolanaRpcClient::new(fail_url)
        .with_fallback(ok_url)
        .with_timeout(Duration::from_secs(2))
        .with_max_retries(1);

    let dummy_key = Keypair::new().pubkey();
    let balance = client.get_balance(&dummy_key).await;

    assert!(
        balance.is_ok(),
        "Should failover to healthy endpoint after 429: {:?}",
        balance.err()
    );
    assert_eq!(balance.unwrap(), 123_456);
}

#[tokio::test]
async fn test_rpc_client_exhaustion_returns_error() {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{}", port);

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let response = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n";
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    let client = SolanaRpcClient::new(url)
        .with_timeout(Duration::from_secs(1))
        .with_max_retries(0);

    let dummy_key = Keypair::new().pubkey();
    let res = client.get_balance(&dummy_key).await;
    assert!(res.is_err());
    match res {
        Err(SolanaError::RpcError { code, message }) => {
            assert_eq!(code, 500);
            assert!(message.contains("500"));
        }
        other => panic!("Expected RpcError 500, got {:?}", other),
    }
}
