//! Phase P6 Acceptance and Lifecycle Integration Test Suite:
//! Production-Safe Transaction Signing Lifecycle.
//!
//! # Verified Requirements:
//! 1. Production signer must be an authentic cryptographic signer (Keypair, KMS, HSM).
//! 2. Disallowed: DevTestSigner, fake signatures, DefaultHasher, deterministic development secrets ([42u8; 32]),
//!    simulated HSM enclave keys, fallback test keys.
//! 3. Explicit signer backend selection; missing configuration fails closed.
//! 4. Verifies public key against configured expected execution authority; rejects mismatch.
//! 5. Fetches current Solana blockhash and records (recent_blockhash, last_valid_block_height).
//! 6. Builds actual transaction.
//! 7. Simulates/preflights according to production policy before submission.
//! 8. Signs only after all deterministic checks pass.
//! 9. Never signs a placeholder instruction.
//! 10. Never logs private key material or serialized secret-key content.

use equity_catalyst_api::{
    config::{Config, ConfigError, Environment},
    engines::execution_signer::{
        is_placeholder_instruction, load_production_signer, DevTestSigner, ExternalSigner,
        HsmSigner, KeypairSigner, KmsSigner, RemoteHsmSigner, SignerError, SigningLifecycle,
        TransactionBuilder, TransactionSignerService, UnavailableSigner, PLACEHOLDER_PROGRAM_ID,
    },
};
use equity_catalyst_solana::rpc::SolanaRpcClient;
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, SeedDerivable, Signer},
};
use std::{fs, fs::File, io::Write, path::PathBuf};

struct TempKeyFile {
    path: PathBuf,
}

impl Drop for TempKeyFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Helper to create a valid on-disk keypair file for testing.
fn create_temp_keypair_file(keypair: &Keypair) -> (TempKeyFile, String) {
    let path = std::env::temp_dir().join(format!("test_key_{}.json", uuid::Uuid::new_v4()));
    let bytes = keypair.to_bytes().to_vec();
    let json = serde_json::to_string(&bytes).expect("Failed to serialize keypair bytes");
    let mut file = File::create(&path).expect("Failed to open temp file for write");
    file.write_all(json.as_bytes())
        .expect("Failed to write keypair json");
    let path_str = path.to_str().unwrap().to_string();
    (TempKeyFile { path }, path_str)
}

fn valid_transfer_instruction(payer: &Pubkey, recipient: &Pubkey) -> Instruction {
    Instruction {
        program_id: Pubkey::new_unique(),
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(*recipient, false),
        ],
        data: vec![2, 0, 0, 0, 100, 0, 0, 0], // Transfer 100 lamports instruction payload
    }
}

// ============================================================================
// TASK 1 & ACCEPTANCE CRITERIA: AUTHENTIC SIGNER VS DISALLOWED SIGNERS
// ============================================================================

#[test]
fn test_authentic_signers_are_production_allowed_and_disallowed_are_rejected() {
    let keypair = Keypair::new();
    let real_keypair_signer = KeypairSigner::from_keypair(keypair);
    assert!(
        real_keypair_signer.is_production_allowed(),
        "Authentic KeypairSigner must be production allowed"
    );

    let kms_signer = KmsSigner::new(
        "arn:aws:kms:us-east-1:123456789012:key/test-prod",
        None,
        Pubkey::new_unique(),
    )
    .unwrap();
    assert!(
        kms_signer.is_production_allowed(),
        "Real KmsSigner must be production allowed"
    );

    let hsm_signer = HsmSigner::new(0, "hsm-execution-authority", Pubkey::new_unique()).unwrap();
    assert!(
        hsm_signer.is_production_allowed(),
        "Real HsmSigner must be production allowed"
    );

    // Disallowed: DevTestSigner
    let dev_signer = DevTestSigner::new_ephemeral();
    assert!(
        !dev_signer.is_production_allowed(),
        "DevTestSigner must NOT be production allowed"
    );

    // Disallowed: simulated HSM enclave key (RemoteHsmSigner)
    let simulated_hsm =
        RemoteHsmSigner::new_with_enclave(Keypair::new(), "http://127.0.0.1:9999", "mock-key-001");
    assert!(
        !simulated_hsm.is_production_allowed(),
        "Simulated HSM enclave signer must NOT be production allowed"
    );

    // Disallowed: deterministic development secret [42u8; 32]
    let dev_seed_keypair = Keypair::from_seed(&[42u8; 32]).unwrap();
    let dev_seed_signer = KeypairSigner::from_keypair(dev_seed_keypair);
    assert!(
        dev_seed_signer.is_dev_secret(),
        "Keypair derived from [42u8; 32] must be flagged as dev secret"
    );
    assert!(
        !dev_seed_signer.is_production_allowed(),
        "Deterministic dev secret [42u8; 32] must NOT be production allowed"
    );

    // Disallowed: UnavailableSigner
    let unavailable = UnavailableSigner::new();
    assert!(
        !unavailable.is_production_allowed(),
        "UnavailableSigner must NOT be production allowed"
    );
}

// ============================================================================
// TASK 2 & 3: CONFIGURATION EXPLICIT SELECTION & FAIL CLOSED
// ============================================================================

#[test]
fn test_missing_or_invalid_signer_configuration_fails_closed() {
    let mut config = Config::mainnet();
    config.database_url = "postgres://user:pass@localhost:5432/db".to_string();
    config.admin_api_key = "sufficiently-long-production-admin-key-12345".to_string();
    config.solana_rpc_url = "https://solana-mainnet.rpc.provider.com".to_string();

    // 1. Missing signer backend fails closed
    config.signer_backend = "".to_string();
    assert_eq!(
        config.validate(),
        Err(ConfigError::MissingSignerBackend(Environment::Mainnet))
    );
    assert!(matches!(
        load_production_signer(&config),
        Err(SignerError::MissingConfiguration(..))
    ));

    // 2. Keypair backend with missing path fails closed
    config.signer_backend = "keypair".to_string();
    config.execution_signer_path = "".to_string();
    assert_eq!(
        config.validate(),
        Err(ConfigError::MissingSignerPath(Environment::Mainnet))
    );
    assert!(matches!(
        load_production_signer(&config),
        Err(SignerError::MissingConfiguration(..))
    ));

    // 3. KMS backend with missing key ID fails closed
    config.signer_backend = "kms".to_string();
    config.kms_key_id = None;
    assert_eq!(
        config.validate(),
        Err(ConfigError::MissingKmsKeyId(Environment::Mainnet))
    );
    assert!(matches!(
        load_production_signer(&config),
        Err(SignerError::MissingConfiguration(..))
    ));

    // 4. HSM backend with missing key label fails closed
    config.signer_backend = "hsm".to_string();
    config.hsm_key_label = None;
    assert_eq!(
        config.validate(),
        Err(ConfigError::MissingHsmKeyLabel(Environment::Mainnet))
    );
    assert!(matches!(
        load_production_signer(&config),
        Err(SignerError::MissingConfiguration(..))
    ));

    // 5. Prohibited test backend in production fails closed
    config.signer_backend = "devtest".to_string();
    assert!(matches!(
        config.validate(),
        Err(ConfigError::InvalidSignerBackend(..))
    ));
    assert!(matches!(
        load_production_signer(&config),
        Err(SignerError::ProductionFallbackProhibited(..))
    ));
}

// ============================================================================
// TASK 4, 5, 6 & ACCEPTANCE CRITERIA: LOAD SIGNER, VERIFY & REJECT MISMATCH
// ============================================================================

#[test]
fn test_load_production_signer_verifies_authority_and_rejects_mismatch() {
    let keypair = Keypair::new();
    let authority = keypair.pubkey();
    let (_tmp, path) = create_temp_keypair_file(&keypair);

    let mut config = Config::mainnet();
    config.signer_backend = "keypair".to_string();
    config.execution_signer_path = path.clone();
    config.expected_execution_authority = Some(authority.to_string());

    // 1. Matching expected authority loads successfully
    let loaded = load_production_signer(&config).expect("Must load valid signer");
    assert_eq!(loaded.pubkey(), authority);
    assert_eq!(loaded.signer_type(), "LocalKeypair");

    // 2. Unexpected public key is strictly rejected
    let wrong_authority = Pubkey::new_unique();
    config.expected_execution_authority = Some(wrong_authority.to_string());
    let mismatch_err = load_production_signer(&config).expect_err("Must reject authority mismatch");
    match mismatch_err {
        SignerError::AuthorityMismatch { expected, actual } => {
            assert_eq!(expected, wrong_authority.to_string());
            assert_eq!(actual, authority.to_string());
        }
        other => panic!("Expected AuthorityMismatch error, got {:?}", other),
    }

    // 3. Unavailable key file fails closed
    config.execution_signer_path = "/nonexistent/path/to/signer.json".to_string();
    config.expected_execution_authority = Some(authority.to_string());
    let missing_file_err =
        load_production_signer(&config).expect_err("Missing keyfile must fail closed");
    assert!(matches!(
        missing_file_err,
        SignerError::KeyFileNotFound { .. }
    ));
}

// ============================================================================
// TASK 7, 8, 9 & ACCEPTANCE CRITERIA: BLOCKHASH & RECORDING & GENUINE SIGNATURE
// ============================================================================

#[tokio::test]
async fn test_valid_transaction_receives_genuine_solana_signature_and_records_blockhash_and_height()
{
    let keypair = Keypair::new();
    let signer = KeypairSigner::from_keypair(keypair);
    let payer = signer.pubkey();
    let recipient = Pubkey::new_unique();

    let recent_blockhash = Hash::new_unique();
    let last_valid_block_height = 284_105_920u64;

    let ix = valid_transfer_instruction(&payer, &recipient);

    // 1. Build actual transaction and verify blockhash and height recording
    let unsigned = TransactionBuilder::build_unsigned(
        &[ix],
        &payer,
        recent_blockhash,
        last_valid_block_height,
    )
    .expect("Unsigned transaction construction must succeed");

    assert_eq!(unsigned.recent_blockhash, recent_blockhash);
    assert_eq!(unsigned.last_valid_block_height, last_valid_block_height);
    assert_eq!(unsigned.payer, payer);

    // 2. Sign transaction with authentic signer
    let signed = TransactionSignerService::sign(&signer, &unsigned)
        .await
        .expect("Signing must succeed");

    // 3. Verify blockhash and last valid block height are preserved in SignedTransaction
    assert_eq!(signed.recent_blockhash, recent_blockhash);
    assert_eq!(signed.last_valid_block_height, last_valid_block_height);
    assert_eq!(signed.signer_pubkey, payer);

    // 4. Genuine Solana signature verification on wire bytes
    let message_bytes = signed.transaction.message_data();
    assert!(
        signed
            .primary_signature
            .verify(&payer.to_bytes(), &message_bytes),
        "Primary signature must be an authentic cryptographic Ed25519 signature"
    );
    assert_ne!(
        signed.primary_signature,
        solana_sdk::signature::Signature::default(),
        "Signature must not be empty or zeroed"
    );

    // Wire bytes are non-empty and validly serializable
    let wire_bytes = signed.to_bytes().expect("Serialization must succeed");
    assert!(!wire_bytes.is_empty());
}

// ============================================================================
// TASK 11 & 12: DETERMINISTIC CHECKS & PLACEHOLDER INSTRUCTION PROHIBITION
// ============================================================================

#[tokio::test]
async fn test_never_sign_placeholder_instruction() {
    let keypair = Keypair::new();
    let signer = KeypairSigner::from_keypair(keypair);
    let payer = signer.pubkey();
    let blockhash = Hash::new_unique();

    // 1. Placeholder by program ID
    let placeholder_ix1 = Instruction {
        program_id: Pubkey::try_from(PLACEHOLDER_PROGRAM_ID).unwrap(),
        accounts: vec![AccountMeta::new(payer, true)],
        data: vec![1, 2, 3],
    };
    assert!(is_placeholder_instruction(&placeholder_ix1));

    let build_err1 = TransactionBuilder::build_unsigned(&[placeholder_ix1], &payer, blockhash, 100)
        .expect_err("Must reject placeholder program ID");
    assert!(matches!(
        build_err1,
        SignerError::PlaceholderInstructionProhibited { .. }
    ));

    // 2. Placeholder by empty accounts and empty data
    let placeholder_ix2 = Instruction {
        program_id: Pubkey::new_unique(),
        accounts: vec![],
        data: vec![],
    };
    assert!(is_placeholder_instruction(&placeholder_ix2));

    let build_err2 = TransactionBuilder::build_unsigned(&[placeholder_ix2], &payer, blockhash, 100)
        .expect_err("Must reject empty accounts and data placeholder instruction");
    assert!(matches!(
        build_err2,
        SignerError::PlaceholderInstructionProhibited { .. }
    ));

    // 3. Placeholder by data sentinel
    let placeholder_ix3 = Instruction {
        program_id: Pubkey::new_unique(),
        accounts: vec![AccountMeta::new(payer, true)],
        data: b"placeholder_instruction_stub".to_vec(),
    };
    assert!(is_placeholder_instruction(&placeholder_ix3));

    let build_err3 = TransactionBuilder::build_unsigned(&[placeholder_ix3], &payer, blockhash, 100)
        .expect_err("Must reject placeholder data sentinel");
    assert!(matches!(
        build_err3,
        SignerError::PlaceholderInstructionProhibited { .. }
    ));
}

#[tokio::test]
async fn test_signer_authority_mismatch_against_payer_rejected_at_signing() {
    let signer_keypair = Keypair::new();
    let signer = KeypairSigner::from_keypair(signer_keypair);

    let different_payer = Pubkey::new_unique();
    let ix = valid_transfer_instruction(&different_payer, &Pubkey::new_unique());

    // Create unsigned transaction with different fee payer
    let unsigned =
        TransactionBuilder::build_unsigned(&[ix], &different_payer, Hash::new_unique(), 500)
            .unwrap();

    // Attempting to sign with a mismatched signer fails closed
    let err = TransactionSignerService::sign(&signer, &unsigned)
        .await
        .expect_err("Signing must fail when payer does not match signer");

    match err {
        SignerError::AuthorityMismatch { expected, actual } => {
            assert_eq!(expected, different_payer.to_string());
            assert_eq!(actual, signer.pubkey().to_string());
        }
        other => panic!("Expected AuthorityMismatch, got {:?}", other),
    }
}

// ============================================================================
// TASK 13 & 14: NEVER LOG PRIVATE KEY MATERIAL OR SERIALIZED SECRET CONTENT
// ============================================================================

#[test]
fn test_never_log_private_key_material_or_serialized_secret_keys() {
    let keypair = Keypair::new();
    let secret_bytes = keypair.to_bytes();
    let keypair_signer = KeypairSigner::from_keypair(keypair);

    let debug_str = format!("{:?}", keypair_signer);
    assert!(
        debug_str.contains("[REDACTED]"),
        "KeypairSigner debug must show [REDACTED]"
    );
    assert!(
        !debug_str.contains(&format!("{:?}", &secret_bytes[..])),
        "Debug output must NEVER contain raw secret key bytes"
    );

    let kms_signer = KmsSigner::new("kms-prod-key-123", None, Pubkey::new_unique()).unwrap();
    let kms_debug = format!("{:?}", kms_signer);
    assert!(!kms_debug.contains("private_key"));
    assert!(!kms_debug.contains("secret"));

    let hsm_signer = HsmSigner::new(1, "hsm-prod-label", Pubkey::new_unique()).unwrap();
    let hsm_debug = format!("{:?}", hsm_signer);
    assert!(!hsm_debug.contains("private_key"));
    assert!(!hsm_debug.contains("secret"));
}

// ============================================================================
// TASK 7, 8, 9, 10, 11: FULL SIGNING LIFECYCLE (RPC, BLOCKHASH, PREFLIGHT, SIGN)
// ============================================================================

#[tokio::test]
async fn test_full_signing_lifecycle_with_preflight_simulation_success() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let expected_hash = Hash::new_unique();
    let expected_height = 295_000_123u64;
    let hash_str = expected_hash.to_string();

    tokio::spawn(async move {
        // Handle getLatestBlockhash and simulateTransaction
        for _ in 0..2 {
            if let Ok((mut socket, _)) = listener.accept().await {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = vec![0u8; 4096];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&buf[..n]);

                let body = if req_str.contains("getLatestBlockhash") {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "context": { "slot": 12345 },
                            "value": {
                                "blockhash": hash_str,
                                "lastValidBlockHeight": expected_height
                            }
                        },
                        "id": 1
                    })
                    .to_string()
                } else if req_str.contains("simulateTransaction") {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "context": { "slot": 12345 },
                            "value": {
                                "err": null,
                                "logs": ["Program success"],
                                "unitsConsumed": 100
                            }
                        },
                        "id": 1
                    })
                    .to_string()
                } else {
                    serde_json::json!({"jsonrpc": "2.0", "result": {}, "id": 1}).to_string()
                };

                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            }
        }
    });

    let rpc_client = SolanaRpcClient::new(format!("http://{}", addr));
    let keypair = Keypair::new();
    let signer = KeypairSigner::from_keypair(keypair);
    let payer = signer.pubkey();
    let recipient = Pubkey::new_unique();
    let ix = valid_transfer_instruction(&payer, &recipient);

    let signed_tx = SigningLifecycle::execute_lifecycle(
        &signer,
        &[ix],
        &payer,
        Some(&payer),
        &rpc_client,
        true, // Preflight enabled
    )
    .await
    .expect("Lifecycle execution with preflight simulation must succeed");

    assert_eq!(signed_tx.recent_blockhash, expected_hash);
    assert_eq!(signed_tx.last_valid_block_height, expected_height);
    assert_eq!(signed_tx.signer_pubkey, payer);

    let message_bytes = signed_tx.transaction.message_data();
    assert!(signed_tx
        .primary_signature
        .verify(&payer.to_bytes(), &message_bytes));
}

#[tokio::test]
async fn test_signing_lifecycle_fails_closed_when_preflight_simulation_errors() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let expected_hash = Hash::new_unique();
    let hash_str = expected_hash.to_string();

    tokio::spawn(async move {
        for _ in 0..2 {
            if let Ok((mut socket, _)) = listener.accept().await {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = vec![0u8; 4096];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&buf[..n]);

                let body = if req_str.contains("getLatestBlockhash") {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "context": { "slot": 12345 },
                            "value": {
                                "blockhash": hash_str,
                                "lastValidBlockHeight": 1000
                            }
                        },
                        "id": 1
                    })
                    .to_string()
                } else {
                    // Simulation error
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "context": { "slot": 12345 },
                            "value": {
                                "err": "InstructionError: InsufficientFunds",
                                "logs": ["Program failed"],
                                "unitsConsumed": 100
                            }
                        },
                        "id": 1
                    })
                    .to_string()
                };

                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            }
        }
    });

    let rpc_client = SolanaRpcClient::new(format!("http://{}", addr));
    let keypair = Keypair::new();
    let signer = KeypairSigner::from_keypair(keypair);
    let payer = signer.pubkey();
    let recipient = Pubkey::new_unique();
    let ix = valid_transfer_instruction(&payer, &recipient);

    let err = SigningLifecycle::execute_lifecycle(
        &signer,
        &[ix],
        &payer,
        Some(&payer),
        &rpc_client,
        true, // Preflight enabled
    )
    .await
    .expect_err("Must fail when preflight simulation fails");

    assert!(
        matches!(err, SignerError::PreflightSimulationFailed(..)),
        "Expected PreflightSimulationFailed, got: {:?}",
        err
    );
}
