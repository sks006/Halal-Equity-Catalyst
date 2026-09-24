//! Isolated execution signer managing on-chain dispatch credentials securely.

use serde::{Deserialize, Serialize};
use solana_sdk::{
    hash::hash,
    pubkey::Pubkey,
    signature::{Keypair, SeedDerivable, Signature},
    signer::Signer,
};
use std::{fs, path::Path, sync::Arc};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    config::Environment,
    engines::execution_signer::{KeypairSigner, SignerError},
};

/// Canonical execution payload representing strictly the deterministic parameters of a financial trade.
/// AI proposals (sentiment, natural language reasoning, prompt instructions) are quarantined
/// and excluded from this payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalExecutionPayload {
    pub execution_id: Uuid,
    pub vault_address: String,
    pub action_type: u8,
    pub input_mint: String,
    pub output_mint: String,
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub execution_seq: u64,
    pub timestamp: i64,
}

impl CanonicalExecutionPayload {
    /// Domain separation prefix to prevent cross-protocol signature replay attacks.
    pub const DOMAIN_SEPARATOR: &'static [u8] = b"EQUITY_CATALYST_EXECUTION_V1";

    /// Produces a deterministic byte framing of the canonical payload.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(Self::DOMAIN_SEPARATOR);
        bytes.extend_from_slice(self.execution_id.as_bytes());
        bytes.extend_from_slice(self.vault_address.as_bytes());
        bytes.push(self.action_type);
        bytes.extend_from_slice(self.input_mint.as_bytes());
        bytes.extend_from_slice(self.output_mint.as_bytes());
        bytes.extend_from_slice(&self.amount_in.to_le_bytes());
        bytes.extend_from_slice(&self.min_amount_out.to_le_bytes());
        bytes.extend_from_slice(&self.execution_seq.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes
    }

    /// Computes the SHA-256 digest of the canonical byte representation.
    pub fn digest(&self) -> [u8; 32] {
        hash(&self.to_canonical_bytes()).to_bytes()
    }
}

/// Isolated cryptographic signer for executing authorized vault decisions.
/// Kept strictly separate from public HTTP request handlers.
#[derive(Clone)]
pub struct ExecutionSigner {
    signer_pubkey: String,
    keypair: Arc<Keypair>,
}

impl std::fmt::Debug for ExecutionSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionSigner")
            .field("signer_pubkey", &self.signer_pubkey)
            .field("keypair", &"[REDACTED]")
            .finish()
    }
}

impl ExecutionSigner {
    /// Loads the execution keypair strictly from disk for production, failing closed if absent or invalid.
    /// NEVER falls back to any hardcoded seed or development keypair.
    pub fn load_production(path_str: &str) -> Result<Self, SignerError> {
        let keypair_signer = KeypairSigner::load_from_path(path_str)?;
        let pubkey_str = keypair_signer.pubkey().to_string();
        Ok(Self {
            signer_pubkey: pubkey_str,
            keypair: Arc::new(keypair_signer.to_solana_keypair()),
        })
    }

    /// Loads the execution signer enforcing environment security policies.
    ///
    /// If `env == Environment::Mainnet`:
    /// Strictly requires a valid keyfile path and FAILS CLOSED if not found.
    ///
    /// If `env != Environment::Mainnet`:
    /// Fails closed unless the keyfile exists or is explicitly configured.
    pub fn load_with_env(path_str: &str, env: Environment) -> Result<Self, SignerError> {
        if env == Environment::Mainnet {
            Self::load_production(path_str).map_err(|e| match e {
                SignerError::KeyFileNotFound { path } => {
                    SignerError::ProductionFallbackProhibited(format!(
                        "Production Mainnet keyfile '{}' not found; fallback to test key is strictly prohibited",
                        path
                    ))
                }
                other => other,
            })
        } else {
            Self::load_production(path_str)
        }
    }

    /// Loads the execution keypair from disk if present; otherwise generates a cryptographically random
    /// ephemeral keypair for dev/testing. NEVER falls back to a hardcoded seed like [42u8; 32].
    pub fn load_or_generate(path_str: &str) -> Self {
        let expanded_path = shellexpand(path_str);
        let path = Path::new(&expanded_path);

        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(bytes) = serde_json::from_str::<Vec<u8>>(&content) {
                    let keypair_opt = if bytes.len() >= 64 {
                        Keypair::from_bytes(&bytes[..64]).ok()
                    } else if bytes.len() >= 32 {
                        Keypair::from_seed(&bytes[..32]).ok()
                    } else {
                        None
                    };

                    if let Some(keypair) = keypair_opt {
                        let pubkey_str = keypair.pubkey().to_string();
                        info!(pubkey = %pubkey_str, "Loaded execution signer keypair from disk");
                        return Self {
                            signer_pubkey: pubkey_str,
                            keypair: Arc::new(keypair),
                        };
                    }
                }
            }
        }

        warn!(
            path = %path_str,
            "Signer file not found; generating fresh cryptographically random ephemeral keypair for dev/test (NO hardcoded seed)"
        );
        let keypair = Keypair::new();
        let pubkey_str = keypair.pubkey().to_string();
        Self {
            signer_pubkey: pubkey_str,
            keypair: Arc::new(keypair),
        }
    }

    /// Explicitly creates a development/test ephemeral signer with a fresh random Keypair.
    pub fn new_dev_ephemeral() -> Self {
        let keypair = Keypair::new();
        let pubkey_str = keypair.pubkey().to_string();
        Self {
            signer_pubkey: pubkey_str,
            keypair: Arc::new(keypair),
        }
    }

    /// Explicitly creates a deterministic test signer with a specified seed.
    /// Strictly for unit tests requiring repeatable signatures.
    pub fn new_dev_test_deterministic(seed: [u8; 32]) -> Self {
        let keypair = Keypair::from_seed(&seed)
            .expect("Deterministic keypair generation from 32-byte seed must succeed");
        let pubkey_str = keypair.pubkey().to_string();
        Self {
            signer_pubkey: pubkey_str,
            keypair: Arc::new(keypair),
        }
    }

    /// Constructs an ExecutionSigner from an existing verified Solana Keypair.
    pub fn from_keypair(keypair: Keypair) -> Self {
        let pubkey_str = keypair.pubkey().to_string();
        Self {
            signer_pubkey: pubkey_str,
            keypair: Arc::new(keypair),
        }
    }

    /// Returns the public key address of the signer as a base58 string.
    pub fn pubkey(&self) -> &str {
        &self.signer_pubkey
    }

    /// Returns the Solana Pubkey of the signer.
    pub fn solana_pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    /// Returns a reference to the underlying Solana Keypair.
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Derives or extracts a Solana SDK Keypair from the loaded key material.
    pub fn to_solana_keypair(&self) -> Result<Keypair, String> {
        Ok(self.keypair.insecure_clone())
    }

    /// Signs an arbitrary message using Ed25519 asymmetric cryptography.
    pub fn sign_message(&self, message: &[u8]) -> Signature {
        self.keypair.sign_message(message)
    }

    /// Generates an Ed25519 digital signature over the SHA-256 digest of a canonical execution payload.
    pub fn sign_canonical_payload(&self, payload: &CanonicalExecutionPayload) -> Signature {
        let digest = payload.digest();
        self.keypair.sign_message(&digest)
    }

    /// Generates a genuine Ed25519 cryptographic signature for an approved execution decision ID.
    /// Returns the standard Base58-encoded Solana signature string.
    pub fn sign_decision(&self, decision_id: &Uuid) -> String {
        let mut msg = Vec::with_capacity(64);
        msg.extend_from_slice(b"EQUITY_CATALYST_DECISION_V1:");
        msg.extend_from_slice(decision_id.as_bytes());

        let sig = self.keypair.sign_message(&msg);
        sig.to_string()
    }

    /// Cryptographically verifies an Ed25519 signature against an expected raw message.
    pub fn verify_signature(pubkey: &Pubkey, message: &[u8], signature: &Signature) -> bool {
        signature.verify(&pubkey.to_bytes(), message)
    }

    /// Cryptographically verifies an Ed25519 signature against a canonical execution payload.
    pub fn verify_canonical_payload(
        pubkey: &Pubkey,
        payload: &CanonicalExecutionPayload,
        signature: &Signature,
    ) -> bool {
        let digest = payload.digest();
        signature.verify(&pubkey.to_bytes(), &digest)
    }

    /// Cryptographically verifies a decision signature string against a decision ID.
    pub fn verify_decision_signature(
        pubkey: &Pubkey,
        decision_id: &Uuid,
        signature_str: &str,
    ) -> bool {
        use std::str::FromStr;
        let Ok(sig) = Signature::from_str(signature_str) else {
            return false;
        };

        let mut msg = Vec::with_capacity(64);
        msg.extend_from_slice(b"EQUITY_CATALYST_DECISION_V1:");
        msg.extend_from_slice(decision_id.as_bytes());

        sig.verify(&pubkey.to_bytes(), &msg)
    }
}

fn shellexpand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_signer_debug_masks_key_material() {
        let signer = ExecutionSigner::load_or_generate("/nonexistent/path/signer.json");
        let debug_str = format!("{:?}", signer);

        assert!(debug_str.contains("[REDACTED]"));
        assert!(debug_str.contains(&signer.signer_pubkey));
        assert!(!debug_str.contains("keypair: Keypair"));
    }

    #[test]
    fn test_pubkey_is_valid_base58_solana_address() {
        let signer = ExecutionSigner::load_or_generate("/nonexistent/path/signer.json");
        let parsed = Pubkey::from_str(signer.pubkey());
        assert!(
            parsed.is_ok(),
            "Signer pubkey should be a valid Base58 Solana address"
        );
        assert_eq!(parsed.unwrap(), signer.solana_pubkey());
    }

    #[test]
    fn test_ed25519_decision_signing_and_verification() {
        let signer = ExecutionSigner::load_or_generate("/nonexistent/path/signer.json");
        let decision_id = Uuid::new_v4();

        let sig_str = signer.sign_decision(&decision_id);

        // Verify it parses as a standard Solana 64-byte Ed25519 signature
        let parsed_sig = Signature::from_str(&sig_str);
        assert!(
            parsed_sig.is_ok(),
            "Output must parse as a valid Solana Signature"
        );

        // Verify valid signature verifies successfully
        let is_valid = ExecutionSigner::verify_decision_signature(
            &signer.solana_pubkey(),
            &decision_id,
            &sig_str,
        );
        assert!(is_valid, "Valid decision signature must pass verification");

        // Verify wrong decision ID fails verification
        let different_decision_id = Uuid::new_v4();
        let is_invalid = ExecutionSigner::verify_decision_signature(
            &signer.solana_pubkey(),
            &different_decision_id,
            &sig_str,
        );
        assert!(!is_invalid, "Tampered decision ID must fail verification");

        // Verify wrong pubkey fails verification
        let other_keypair = Keypair::new();
        let is_wrong_key = ExecutionSigner::verify_decision_signature(
            &other_keypair.pubkey(),
            &decision_id,
            &sig_str,
        );
        assert!(
            !is_wrong_key,
            "Signature must fail with different public key"
        );
    }

    #[test]
    fn test_ed25519_canonical_payload_signing_and_tamper_detection() {
        let signer = ExecutionSigner::load_or_generate("/nonexistent/path/signer.json");
        let payload = CanonicalExecutionPayload {
            execution_id: Uuid::new_v4(),
            vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4".to_string(),
            action_type: 1,
            input_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            output_mint: "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R".to_string(),
            amount_in: 50_000_000_000,
            min_amount_out: 387_000_000,
            execution_seq: 42,
            timestamp: 1726912800,
        };

        let sig = signer.sign_canonical_payload(&payload);

        // Positive verification
        assert!(ExecutionSigner::verify_canonical_payload(
            &signer.solana_pubkey(),
            &payload,
            &sig,
        ));

        // Tamper test 1: Amount modified
        let mut tampered_amount = payload.clone();
        tampered_amount.amount_in = 50_000_000_001;
        assert!(
            !ExecutionSigner::verify_canonical_payload(
                &signer.solana_pubkey(),
                &tampered_amount,
                &sig
            ),
            "Tampered amount_in must fail verification"
        );

        // Tamper test 2: Mint swapped
        let mut tampered_mint = payload.clone();
        tampered_mint.output_mint = "So11111111111111111111111111111111111111112".to_string();
        assert!(
            !ExecutionSigner::verify_canonical_payload(
                &signer.solana_pubkey(),
                &tampered_mint,
                &sig
            ),
            "Tampered mint must fail verification"
        );

        // Tamper test 3: Timestamp changed
        let mut tampered_time = payload.clone();
        tampered_time.timestamp += 1;
        assert!(
            !ExecutionSigner::verify_canonical_payload(
                &signer.solana_pubkey(),
                &tampered_time,
                &sig
            ),
            "Tampered timestamp must fail verification"
        );
    }
}
