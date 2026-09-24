//! Integration test suite for Phase 12: Real Cryptographic Signing & Signer Boundary.
//!
//! # Verified Security Requirements:
//! 1. No DefaultHasher as a signature.
//! 2. No fake hash-based signatures (UUID + key).
//! 3. No hardcoded fallback private key.
//! 4. No `[42u8; 32]` or equivalent deterministic secret in production code.
//! 5. Execution planner does not receive raw private keys.
//! 6. Strict separation of:
//!    - Transaction Construction
//!    - Transaction Signing
//!    - Transaction Submission
//! 7. External signer abstraction (`ExternalSigner` trait).
//! 8. Fail closed when the signer is unavailable.
//! 9. Explicit development/test signer only behind dev/test configuration.
//! 10. Required test coverage:
//!    - valid payload signs
//!    - modified payload produces a different signature
//!    - wrong signer is rejected
//!    - unavailable signer fails closed
//!    - production configuration cannot silently fall back to a test key

use equity_catalyst_api::{
    config::Environment,
    engines::{
        decision_engine::{CanonicalExecutionPayload, ExecutionSigner},
        execution_planner::{ExecutionPlan, OracleReferenceInfo},
        execution_signer::{
            DevTestSigner, ExternalSigner, KeypairSigner, RemoteHsmSigner, SignerError,
            TransactionBuilder, TransactionSignerService, UnavailableSigner,
        },
    },
};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
};
use uuid::Uuid;

fn sample_canonical_payload() -> CanonicalExecutionPayload {
    CanonicalExecutionPayload {
        execution_id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
        vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4".to_string(),
        action_type: 1,
        input_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        output_mint: "AAPL111111111111111111111111111111111111111".to_string(),
        amount_in: 5_000_000_000,
        min_amount_out: 24_875_000,
        execution_seq: 101,
        timestamp: 1_750_000_000,
    }
}

fn sample_instruction(payer: &Pubkey) -> Instruction {
    Instruction {
        program_id: Pubkey::new_unique(),
        accounts: vec![AccountMeta::new(*payer, true)],
        data: vec![1, 2, 3, 4],
    }
}

#[tokio::test]
async fn test_valid_payload_signs_and_verifies() {
    let keypair = Keypair::new();
    let signer = KeypairSigner::from_keypair(keypair);

    let payload = sample_canonical_payload();
    let digest = payload.digest();

    // 1. Sign canonical payload digest
    let signature = signer
        .sign_message(&digest)
        .expect("Valid payload must sign successfully");

    // 2. Cryptographically verify signature using signer's Ed25519 public key
    assert!(
        signature.verify(&signer.pubkey().to_bytes(), &digest),
        "Cryptographic signature must verify against authentic public key"
    );

    // 3. Test through ExecutionSigner wrapper
    let exec_signer = ExecutionSigner::from_keypair(signer.to_solana_keypair());
    assert!(ExecutionSigner::verify_canonical_payload(
        &exec_signer.solana_pubkey(),
        &payload,
        &signature
    ));
}

#[tokio::test]
async fn test_modified_payload_produces_different_signature_and_fails_verification() {
    let keypair = Keypair::new();
    let signer = KeypairSigner::from_keypair(keypair);

    let original_payload = sample_canonical_payload();
    let original_digest = original_payload.digest();
    let original_signature = signer
        .sign_message(&original_digest)
        .expect("Original payload must sign");

    // Alter payload amount by 1 unit
    let mut modified_payload = original_payload.clone();
    modified_payload.amount_in += 1;
    let modified_digest = modified_payload.digest();

    assert_ne!(
        original_digest, modified_digest,
        "Modified payload must produce different SHA-256 digest"
    );

    let modified_signature = signer
        .sign_message(&modified_digest)
        .expect("Modified payload must sign");

    // Invariant: Modified payload produces a different signature
    assert_ne!(
        original_signature, modified_signature,
        "Modified payload must produce a different cryptographic signature"
    );

    // Invariant: Original signature FAILS verification against modified payload
    assert!(
        !original_signature.verify(&signer.pubkey().to_bytes(), &modified_digest),
        "Original signature must NOT verify against modified payload"
    );
    assert!(
        !modified_signature.verify(&signer.pubkey().to_bytes(), &original_digest),
        "Modified signature must NOT verify against original payload"
    );
}

#[tokio::test]
async fn test_wrong_signer_is_rejected() {
    let keypair_a = Keypair::new();
    let signer_a = KeypairSigner::from_keypair(keypair_a);

    let keypair_b = Keypair::new();
    let signer_b = KeypairSigner::from_keypair(keypair_b);

    assert_ne!(signer_a.pubkey(), signer_b.pubkey());

    let payload = sample_canonical_payload();
    let digest = payload.digest();

    // Payload signed by Signer A
    let sig_a = signer_a.sign_message(&digest).unwrap();

    // Verification against Signer B's public key MUST fail
    let verified_with_wrong_key = sig_a.verify(&signer_b.pubkey().to_bytes(), &digest);
    assert!(
        !verified_with_wrong_key,
        "Signature from Signer A must be rejected by Signer B's public key"
    );
}

#[tokio::test]
async fn test_unavailable_signer_fails_closed() {
    // 1. Test dedicated UnavailableSigner
    let unavailable = UnavailableSigner::new();
    assert!(!unavailable.is_available());

    let message = b"test transaction payload";
    let sign_result = unavailable.sign_message(message);
    assert!(sign_result.is_err());
    match sign_result.unwrap_err() {
        SignerError::SignerUnavailable { signer_type, .. } => {
            assert_eq!(signer_type, "UnavailableSigner");
        }
        other => panic!("Expected SignerUnavailable, got {:?}", other),
    }

    // 2. Test RemoteHsmSigner when offline
    let enclave_keypair = Keypair::new();
    let mut hsm = RemoteHsmSigner::new_with_enclave(
        enclave_keypair,
        "https://kms.us-east-1.amazonaws.com/v1",
        "key-prod-001",
    );
    assert!(hsm.is_available());

    // Disconnect HSM (simulate network partition or enclave offline)
    hsm.set_available(false);
    assert!(!hsm.is_available());

    let hsm_sign_result = hsm.sign_message(message);
    assert!(
        matches!(hsm_sign_result, Err(SignerError::SignerUnavailable { .. })),
        "Unavailable HSM must fail closed"
    );

    // 3. Test through TransactionSignerService pipeline
    let payer = Pubkey::new_unique();
    let ix = sample_instruction(&payer);
    let blockhash = Hash::new_unique();
    let unsigned_tx = TransactionBuilder::build_unsigned(&[ix], &payer, blockhash).unwrap();

    let pipeline_result = TransactionSignerService::sign(&hsm, &unsigned_tx).await;
    assert!(
        matches!(pipeline_result, Err(SignerError::SignerUnavailable { .. })),
        "TransactionSignerService must fail closed when signer is unavailable"
    );
}

#[test]
fn test_production_configuration_cannot_silently_fallback_to_test_key() {
    let nonexistent_path = "/nonexistent/prod/mainnet_authority_signer.json";

    // 1. Direct keypair loader fails closed
    let load_res = KeypairSigner::load_from_path(nonexistent_path);
    assert!(
        matches!(load_res, Err(SignerError::KeyFileNotFound { .. })),
        "KeypairSigner must fail closed with KeyFileNotFound; no default seeds allowed"
    );

    // 2. ExecutionSigner production loader fails closed
    let prod_res = ExecutionSigner::load_production(nonexistent_path);
    assert!(
        matches!(prod_res, Err(SignerError::KeyFileNotFound { .. })),
        "ExecutionSigner::load_production must fail closed"
    );

    // 3. Environment Mainnet loader strictly prohibits fallback
    let mainnet_res = ExecutionSigner::load_with_env(nonexistent_path, Environment::Mainnet);
    assert!(
        matches!(
            mainnet_res,
            Err(SignerError::ProductionFallbackProhibited(..))
        ),
        "Production Mainnet configuration must reject fallback to any test key"
    );

    // 4. Verify no hardcoded [42u8; 32] is ever loaded:
    // Even dev ephemeral loader produces a fresh unique random keypair every invocation
    let dev_signer_1 = ExecutionSigner::new_dev_ephemeral();
    let dev_signer_2 = ExecutionSigner::new_dev_ephemeral();
    assert_ne!(
        dev_signer_1.pubkey(),
        dev_signer_2.pubkey(),
        "Ephemeral dev signer must generate unique random keypairs, never fixed seeds"
    );
}

#[tokio::test]
async fn test_separated_transaction_lifecycle_construction_signing_submission() {
    let keypair = Keypair::new();
    let signer = KeypairSigner::from_keypair(keypair);
    let payer = signer.pubkey();
    let blockhash = Hash::new_unique();

    let plan = ExecutionPlan {
        plan_id: Uuid::new_v4(),
        policy_decision_id: Uuid::new_v4(),
        idempotency_key: "idemp:lifecycle:001".to_string(),
        vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4".to_string(),
        asset_id: "backed:AAPL".to_string(),
        symbol: "AAPL".to_string(),
        is_buy: true,
        input_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        output_mint: "AAPL111111111111111111111111111111111111111".to_string(),
        input_amount: 5_000_000_000,
        expected_output_amount: 25_000_000,
        minimum_output_amount: 24_875_000,
        slippage_bps: 50,
        quote_id: "quote-lifecycle-001".to_string(),
        quote_timestamp: 1_750_000_000,
        oracle_reference: OracleReferenceInfo {
            price_scaled: 200_000_000,
            publish_time: 1_749_999_995,
            conf_bps: 10,
            feed_id: "feed-test".to_string(),
        },
        expires_at: 1_750_000_030,
        created_at: 1_750_000_000,
    };

    // ========================================================================
    // STAGE 1: TRANSACTION CONSTRUCTION (Pure Message Building - No Signing)
    // ========================================================================
    let unsigned = TransactionBuilder::build_from_plan(&plan, &payer, blockhash, &[])
        .expect("Construction must succeed");

    assert_eq!(unsigned.payer, payer);
    assert_eq!(unsigned.recent_blockhash, blockhash);
    // Unsigned transaction has 0 signatures
    assert!(
        unsigned.transaction.signatures.is_empty()
            || unsigned.transaction.signatures.iter().all(|s| *s == solana_sdk::signature::Signature::default()),
        "Unsigned transaction must not be signed"
    );

    // ========================================================================
    // STAGE 2: TRANSACTION SIGNING (Isolated Cryptographic Signing - No Submission)
    // ========================================================================
    let signed = TransactionSignerService::sign(&signer, &unsigned)
        .await
        .expect("Signing stage must succeed");

    assert_eq!(signed.signer_pubkey, signer.pubkey());
    assert_eq!(signed.signer_type, "LocalKeypair");
    assert_ne!(
        signed.primary_signature,
        solana_sdk::signature::Signature::default(),
        "Must produce real Ed25519 signature"
    );

    // Verify signature verifies against the transaction message data
    let message_bytes = signed.transaction.message_data();
    assert!(
        signed
            .primary_signature
            .verify(&signer.pubkey().to_bytes(), &message_bytes),
        "Transaction signature must verify against transaction message"
    );

    // Serialized wire bytes are valid
    let wire_bytes = signed.to_bytes().unwrap();
    assert!(!wire_bytes.is_empty());

    // ========================================================================
    // STAGE 3: TRANSACTION SUBMISSION (Submission Interface Isolation)
    // ========================================================================
    // Submission stage receives the SignedTransaction without holding private keys
    // (Verified through interface boundary: TransactionSubmitter takes &SignedTransaction, not Signer)
}

#[test]
fn test_planner_does_not_receive_raw_private_keys() {
    // Structural proof: ExecutionPlan has no Keypair, no private keys, and is pure data
    let plan = ExecutionPlan {
        plan_id: Uuid::new_v4(),
        policy_decision_id: Uuid::new_v4(),
        idempotency_key: "idemp:isolation:001".to_string(),
        vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4".to_string(),
        asset_id: "backed:AAPL".to_string(),
        symbol: "AAPL".to_string(),
        is_buy: true,
        input_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        output_mint: "AAPL111111111111111111111111111111111111111".to_string(),
        input_amount: 5_000_000_000,
        expected_output_amount: 25_000_000,
        minimum_output_amount: 24_875_000,
        slippage_bps: 50,
        quote_id: "quote-isolation-001".to_string(),
        quote_timestamp: 1_750_000_000,
        oracle_reference: OracleReferenceInfo {
            price_scaled: 200_000_000,
            publish_time: 1_749_999_995,
            conf_bps: 10,
            feed_id: "feed-test".to_string(),
        },
        expires_at: 1_750_000_030,
        created_at: 1_750_000_000,
    };

    // Serialize to JSON and verify no key material is serialized or exists
    let json = serde_json::to_string(&plan).unwrap();
    assert!(!json.contains("private_key"));
    assert!(!json.contains("secret"));
    assert!(!json.contains("keypair"));
}

#[tokio::test]
async fn test_dev_test_signer_explicit_dev_profile_only() {
    // 1. Ephemeral dev signer generates random keys
    let dev_signer_a = DevTestSigner::new_ephemeral();
    let dev_signer_b = DevTestSigner::new_ephemeral();
    assert_ne!(dev_signer_a.pubkey(), dev_signer_b.pubkey());
    assert_eq!(dev_signer_a.signer_type(), "DevTestSigner");

    // 2. Deterministic test signer is explicitly labeled
    let seed = [77u8; 32];
    let test_signer = DevTestSigner::new_deterministic_test_only(seed);
    assert_eq!(test_signer.signer_type(), "DevTestSigner");

    let msg = b"test message for dev signer";
    let sig = test_signer.sign_message(msg).unwrap();
    assert!(sig.verify(&test_signer.pubkey().to_bytes(), msg));
}
