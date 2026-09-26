//! External signer abstraction and cryptographic implementations.
//!
//! Provides an isolated trait boundary for Ed25519 signing, supporting local filesystem keypairs,
//! simulated remote HSM / KMS (AWS KMS, GCP Cloud KMS, HashiCorp Vault), and explicit dev/test signers.
//!
//! # Security Guarantees:
//! - No `DefaultHasher` or fake hash-based signatures.
//! - No hardcoded fallback seeds (`[42u8; 32]` or equivalent) in production paths.
//! - Fails closed if the key file is missing, corrupted, or if an external signer is offline.
//! - Strictly separates transaction construction from signing.

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, SeedDerivable, Signature, Signer},
    transaction::Transaction,
};
use std::{fs, path::Path, str::FromStr, sync::Arc};
use tracing::info;

use super::error::SignerError;

/// Core external signer abstraction for Solana transaction execution.
///
/// Dyn-compatible trait allowing dynamic dispatch across diverse hardware security
/// modules, cloud KMS providers, and local filesystem keypairs.
pub trait ExternalSigner: Send + Sync + std::fmt::Debug {
    /// Returns the public key of the signer.
    fn pubkey(&self) -> Pubkey;

    /// Returns the Base58 string representation of the public key.
    fn pubkey_string(&self) -> String {
        self.pubkey().to_string()
    }

    /// Signs an arbitrary message using Ed25519 asymmetric cryptography.
    fn sign_message(&self, message: &[u8]) -> Result<Signature, SignerError>;

    /// Cryptographically signs an unsigned Solana transaction.
    fn sign_transaction(&self, tx: &mut Transaction) -> Result<(), SignerError>;

    /// Indicates whether the signer is currently reachable and operational.
    fn is_available(&self) -> bool;

    /// Identifier of the underlying signer provider (e.g. "LocalKeypair", "RemoteHsm", "AwsKms").
    fn signer_type(&self) -> &'static str;

    /// Indicates whether this signer is an authentic, production-eligible signer.
    /// Returns false for DevTestSigner, simulated HSM enclave keys, or test keys.
    fn is_production_allowed(&self) -> bool;
}

// ============================================================================
// 1. LOCAL KEYPAIR SIGNER (Strict, Fail-Closed File Loader)
// ============================================================================

/// Real cryptographic signer backed by a local Ed25519 keypair file.
///
/// Fails closed: if the specified key file does not exist or is invalid,
/// it NEVER falls back to any hardcoded or deterministic default seed.
#[derive(Clone)]
pub struct KeypairSigner {
    keypair: Arc<Keypair>,
    pubkey: Pubkey,
    path: Option<String>,
}

impl std::fmt::Debug for KeypairSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeypairSigner")
            .field("pubkey", &self.pubkey.to_string())
            .field("keypair", &"[REDACTED]")
            .field("path", &self.path)
            .finish()
    }
}

impl KeypairSigner {
    /// Loads an Ed25519 keypair from a filesystem path, strictly failing closed if absent or invalid.
    pub fn load_from_path(path_str: &str) -> Result<Self, SignerError> {
        let expanded = shellexpand(path_str);
        let path = Path::new(&expanded);

        if !path.exists() {
            return Err(SignerError::KeyFileNotFound {
                path: path_str.to_string(),
            });
        }

        let content = fs::read_to_string(path).map_err(|e| SignerError::InvalidKeyFormat {
            path: path_str.to_string(),
            details: format!("Failed to read file: {}", e),
        })?;

        let bytes = serde_json::from_str::<Vec<u8>>(&content).map_err(|e| {
            SignerError::InvalidKeyFormat {
                path: path_str.to_string(),
                details: format!("JSON byte array deserialization failed: {}", e),
            }
        })?;

        let keypair = if bytes.len() >= 64 {
            Keypair::from_bytes(&bytes[..64]).map_err(|e| SignerError::InvalidKeyFormat {
                path: path_str.to_string(),
                details: format!("Keypair decoding failed: {}", e),
            })?
        } else if bytes.len() >= 32 {
            Keypair::from_seed(&bytes[..32]).map_err(|e| SignerError::InvalidKeyFormat {
                path: path_str.to_string(),
                details: format!("Seed decoding failed: {}", e),
            })?
        } else {
            return Err(SignerError::InvalidKeyFormat {
                path: path_str.to_string(),
                details: format!("Expected at least 32 bytes, got {}", bytes.len()),
            });
        };

        let pubkey = keypair.pubkey();
        info!(
            pubkey = %pubkey,
            path = %path_str,
            "Loaded authentic cryptographic keypair from disk"
        );

        Ok(Self {
            keypair: Arc::new(keypair),
            pubkey,
            path: Some(path_str.to_string()),
        })
    }

    /// Wraps an existing, verified Keypair in an `Arc`.
    pub fn from_keypair(keypair: Keypair) -> Self {
        let pubkey = keypair.pubkey();
        Self {
            keypair: Arc::new(keypair),
            pubkey,
            path: None,
        }
    }

    /// Inherent getter for the public key.
    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    /// Returns a clone of the inner Solana Keypair for integration with legacy interfaces.
    pub fn to_solana_keypair(&self) -> Keypair {
        self.keypair.insecure_clone()
    }

    /// Checks if this keypair was derived from known deterministic development secrets
    /// (e.g. `[42u8; 32]`, all zeros, all ones), which are prohibited in production.
    pub fn is_dev_secret(&self) -> bool {
        if let Ok(dev_42) = Keypair::from_seed(&[42u8; 32]) {
            if self.pubkey == dev_42.pubkey() {
                return true;
            }
        }
        if let Ok(dev_zero) = Keypair::from_seed(&[0u8; 32]) {
            if self.pubkey == dev_zero.pubkey() {
                return true;
            }
        }
        if let Ok(dev_one) = Keypair::from_seed(&[1u8; 32]) {
            if self.pubkey == dev_one.pubkey() {
                return true;
            }
        }
        false
    }
}

impl ExternalSigner for KeypairSigner {
    fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    fn sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        Ok(self.keypair.sign_message(message))
    }

    fn sign_transaction(&self, tx: &mut Transaction) -> Result<(), SignerError> {
        if tx.message.recent_blockhash == solana_sdk::hash::Hash::default() {
            return Err(SignerError::InvalidTransactionState(
                "Transaction blockhash must be set before signing".to_string(),
            ));
        }
        let blockhash = tx.message.recent_blockhash;
        tx.try_sign(&[&*self.keypair], blockhash).map_err(|e| {
            SignerError::SigningFailed(format!("Solana transaction signing failed: {}", e))
        })
    }

    fn is_available(&self) -> bool {
        true
    }

    fn signer_type(&self) -> &'static str {
        "LocalKeypair"
    }

    fn is_production_allowed(&self) -> bool {
        !self.is_dev_secret()
    }
}

// ============================================================================
// 2. REMOTE HSM / KMS SIGNER (Cloud KMS, Vault, Hardware Security Module)
// ============================================================================

/// Signer implementation representing an external Hardware Security Module (HSM) or Cloud KMS.
///
/// Simulates enterprise Vault / AWS KMS / GCP KMS signing where private key material
/// NEVER leaves the secure enclave.
#[derive(Clone)]
pub struct RemoteHsmSigner {
    pubkey: Pubkey,
    endpoint: String,
    key_id: String,
    available: bool,
    // Secure enclave backing for cryptographic simulation
    enclave_keypair: Option<Arc<Keypair>>,
}

impl std::fmt::Debug for RemoteHsmSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteHsmSigner")
            .field("pubkey", &self.pubkey.to_string())
            .field("endpoint", &self.endpoint)
            .field("key_id", &self.key_id)
            .field("available", &self.available)
            .finish()
    }
}

impl RemoteHsmSigner {
    /// Creates an HSM signer backed by an enclave keypair.
    pub fn new_with_enclave(
        enclave_keypair: Keypair,
        endpoint: impl Into<String>,
        key_id: impl Into<String>,
    ) -> Self {
        let pubkey = enclave_keypair.pubkey();
        Self {
            pubkey,
            endpoint: endpoint.into(),
            key_id: key_id.into(),
            available: true,
            enclave_keypair: Some(Arc::new(enclave_keypair)),
        }
    }

    /// Inherent getter for the public key.
    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    /// Sets the availability status of the remote signer.
    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }
}

impl ExternalSigner for RemoteHsmSigner {
    fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    fn sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        if !self.available {
            return Err(SignerError::SignerUnavailable {
                signer_type: "RemoteHsm",
                reason: format!("HSM endpoint '{}' is unreachable", self.endpoint),
            });
        }

        match &self.enclave_keypair {
            Some(keypair) => Ok(keypair.sign_message(message)),
            None => Err(SignerError::SigningFailed(
                "Enclave key material not initialized".to_string(),
            )),
        }
    }

    fn sign_transaction(&self, tx: &mut Transaction) -> Result<(), SignerError> {
        if !self.available {
            return Err(SignerError::SignerUnavailable {
                signer_type: "RemoteHsm",
                reason: format!("HSM endpoint '{}' is unreachable", self.endpoint),
            });
        }

        if tx.message.recent_blockhash == solana_sdk::hash::Hash::default() {
            return Err(SignerError::InvalidTransactionState(
                "Transaction blockhash must be set before signing".to_string(),
            ));
        }

        let keypair = self
            .enclave_keypair
            .as_ref()
            .ok_or_else(|| SignerError::SigningFailed("Enclave key not available".to_string()))?;

        let blockhash = tx.message.recent_blockhash;
        tx.try_sign(&[&**keypair], blockhash).map_err(|e| {
            SignerError::SigningFailed(format!("HSM transaction signing failed: {}", e))
        })
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn signer_type(&self) -> &'static str {
        "RemoteHsm"
    }

    fn is_production_allowed(&self) -> bool {
        // Simulated enclave key is strictly prohibited in production
        false
    }
}

// ============================================================================
// 3. UNAVAILABLE SIGNER (Guaranteed Fail-Closed Testing)
// ============================================================================

/// A signer that always reports offline/unavailable and fails closed on all operations.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSigner {
    dummy_pubkey: Pubkey,
}

impl UnavailableSigner {
    pub fn new() -> Self {
        Self {
            dummy_pubkey: Pubkey::new_unique(),
        }
    }

    pub fn pubkey(&self) -> Pubkey {
        self.dummy_pubkey
    }
}

impl ExternalSigner for UnavailableSigner {
    fn pubkey(&self) -> Pubkey {
        self.dummy_pubkey
    }

    fn sign_message(&self, _message: &[u8]) -> Result<Signature, SignerError> {
        Err(SignerError::SignerUnavailable {
            signer_type: "UnavailableSigner",
            reason: "Signer hardware is offline or disconnected".to_string(),
        })
    }

    fn sign_transaction(&self, _tx: &mut Transaction) -> Result<(), SignerError> {
        Err(SignerError::SignerUnavailable {
            signer_type: "UnavailableSigner",
            reason: "Signer hardware is offline or disconnected".to_string(),
        })
    }

    fn is_available(&self) -> bool {
        false
    }

    fn signer_type(&self) -> &'static str {
        "UnavailableSigner"
    }

    fn is_production_allowed(&self) -> bool {
        false
    }
}

// ============================================================================
// 4. DEVELOPMENT / TEST SIGNER (Explicitly Isolated Behind Dev Profile)
// ============================================================================

/// Development and test signer clearly separated from production configurations.
#[derive(Clone)]
pub struct DevTestSigner {
    keypair: Arc<Keypair>,
    pubkey: Pubkey,
    description: &'static str,
}

impl std::fmt::Debug for DevTestSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DevTestSigner")
            .field("pubkey", &self.pubkey.to_string())
            .field("description", &self.description)
            .field("keypair", &"[REDACTED]")
            .finish()
    }
}

impl DevTestSigner {
    /// Generates a fresh random Ed25519 keypair for ephemeral unit testing.
    /// Does NOT use any hardcoded or known seed.
    pub fn new_ephemeral() -> Self {
        let keypair = Keypair::new();
        let pubkey = keypair.pubkey();
        Self {
            keypair: Arc::new(keypair),
            pubkey,
            description: "Ephemeral random dev/test keypair",
        }
    }

    /// Creates a deterministic test signer strictly for tests that require reproducible public keys.
    /// Explicitly marked test-only.
    pub fn new_deterministic_test_only(seed: [u8; 32]) -> Self {
        let keypair =
            Keypair::from_seed(&seed).expect("Valid 32-byte seed creates valid Ed25519 keypair");
        let pubkey = keypair.pubkey();
        Self {
            keypair: Arc::new(keypair),
            pubkey,
            description: "Deterministic test-only keypair",
        }
    }

    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
    }
}

impl ExternalSigner for DevTestSigner {
    fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    fn sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        Ok(self.keypair.sign_message(message))
    }

    fn sign_transaction(&self, tx: &mut Transaction) -> Result<(), SignerError> {
        if tx.message.recent_blockhash == solana_sdk::hash::Hash::default() {
            return Err(SignerError::InvalidTransactionState(
                "Transaction blockhash must be set before signing".to_string(),
            ));
        }
        let blockhash = tx.message.recent_blockhash;
        tx.try_sign(&[&*self.keypair], blockhash)
            .map_err(|e| SignerError::SigningFailed(format!("Dev/Test signing failed: {}", e)))
    }

    fn is_available(&self) -> bool {
        true
    }

    fn signer_type(&self) -> &'static str {
        "DevTestSigner"
    }

    fn is_production_allowed(&self) -> bool {
        // Dev/Test signer is strictly prohibited in production
        false
    }
}

// ============================================================================
// 5. CLOUD KMS SIGNER (AWS KMS, GCP Cloud KMS, HashiCorp Vault)
// ============================================================================

/// Authentic cryptographic signer backed by Cloud Key Management Service.
///
/// In production, private key material NEVER leaves the secure cloud KMS boundary.
/// Asymmetric Ed25519 signing requests are verified and executed remotely.
#[derive(Clone)]
pub struct KmsSigner {
    pubkey: Pubkey,
    key_id: String,
    endpoint: Option<String>,
    available: bool,
    signing_client: Option<Arc<dyn Fn(&[u8]) -> Result<Signature, SignerError> + Send + Sync>>,
}

impl std::fmt::Debug for KmsSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KmsSigner")
            .field("pubkey", &self.pubkey.to_string())
            .field("key_id", &self.key_id)
            .field("endpoint", &self.endpoint)
            .field("available", &self.available)
            .finish()
    }
}

impl KmsSigner {
    /// Creates a production KMS signer with explicit key ID, optional endpoint, and public key.
    pub fn new(
        key_id: impl Into<String>,
        endpoint: Option<String>,
        pubkey: Pubkey,
    ) -> Result<Self, SignerError> {
        let key_id_str = key_id.into();
        if key_id_str.trim().is_empty() {
            return Err(SignerError::MissingConfiguration(
                "KMS key ID cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            pubkey,
            key_id: key_id_str,
            endpoint,
            available: true,
            signing_client: None,
        })
    }

    /// Creates an authentic KMS signer with an active cryptographic signing callback/client.
    pub fn new_with_signer(
        pubkey: Pubkey,
        key_id: impl Into<String>,
        endpoint: Option<String>,
        signing_client: Arc<dyn Fn(&[u8]) -> Result<Signature, SignerError> + Send + Sync>,
    ) -> Self {
        Self {
            pubkey,
            key_id: key_id.into(),
            endpoint,
            available: true,
            signing_client: Some(signing_client),
        }
    }

    /// Sets the availability state of the KMS connection.
    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }

    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
    }
}

impl ExternalSigner for KmsSigner {
    fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    fn sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        if !self.available {
            return Err(SignerError::SignerUnavailable {
                signer_type: "KmsSigner",
                reason: format!("KMS key '{}' endpoint is unreachable", self.key_id),
            });
        }

        if let Some(client) = &self.signing_client {
            client(message)
        } else {
            Err(SignerError::SignerUnavailable {
                signer_type: "KmsSigner",
                reason: format!("KMS provider for key '{}' is not connected", self.key_id),
            })
        }
    }

    fn sign_transaction(&self, tx: &mut Transaction) -> Result<(), SignerError> {
        if !self.available {
            return Err(SignerError::SignerUnavailable {
                signer_type: "KmsSigner",
                reason: format!("KMS key '{}' endpoint is unreachable", self.key_id),
            });
        }

        if tx.message.recent_blockhash == solana_sdk::hash::Hash::default() {
            return Err(SignerError::InvalidTransactionState(
                "Transaction blockhash must be set before signing".to_string(),
            ));
        }

        let message_data = tx.message_data();
        let sig = self.sign_message(&message_data)?;

        let signer_index = tx
            .message
            .account_keys
            .iter()
            .position(|k| *k == self.pubkey)
            .ok_or_else(|| {
                SignerError::SigningFailed(format!(
                    "Signer pubkey {} not found in transaction accounts",
                    self.pubkey
                ))
            })?;

        if tx.signatures.len() <= signer_index {
            tx.signatures.resize(
                tx.message.header.num_required_signatures as usize,
                Signature::default(),
            );
        }
        tx.signatures[signer_index] = sig;

        Ok(())
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn signer_type(&self) -> &'static str {
        "KmsSigner"
    }

    fn is_production_allowed(&self) -> bool {
        true
    }
}

// ============================================================================
// 6. HARDWARE SECURITY MODULE (HSM) SIGNER (PKCS#11, CloudHSM, YubiHSM)
// ============================================================================

/// Authentic cryptographic signer backed by a Hardware Security Module.
///
/// Private keys reside entirely on tamper-resistant cryptographic hardware and cannot be extracted.
#[derive(Clone)]
pub struct HsmSigner {
    pubkey: Pubkey,
    slot: u64,
    key_label: String,
    available: bool,
    signing_client: Option<Arc<dyn Fn(&[u8]) -> Result<Signature, SignerError> + Send + Sync>>,
}

impl std::fmt::Debug for HsmSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HsmSigner")
            .field("pubkey", &self.pubkey.to_string())
            .field("slot", &self.slot)
            .field("key_label", &self.key_label)
            .field("available", &self.available)
            .finish()
    }
}

impl HsmSigner {
    /// Creates an authentic HSM signer referencing a specific hardware slot and key label.
    pub fn new(
        slot: u64,
        key_label: impl Into<String>,
        pubkey: Pubkey,
    ) -> Result<Self, SignerError> {
        let label = key_label.into();
        if label.trim().is_empty() {
            return Err(SignerError::MissingConfiguration(
                "HSM key label cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            pubkey,
            slot,
            key_label: label,
            available: true,
            signing_client: None,
        })
    }

    /// Creates an authentic HSM signer with an active cryptographic signing callback/client.
    pub fn new_with_signer(
        pubkey: Pubkey,
        slot: u64,
        key_label: impl Into<String>,
        signing_client: Arc<dyn Fn(&[u8]) -> Result<Signature, SignerError> + Send + Sync>,
    ) -> Self {
        Self {
            pubkey,
            slot,
            key_label: key_label.into(),
            available: true,
            signing_client: Some(signing_client),
        }
    }

    /// Sets the availability state of the HSM hardware connection.
    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }

    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
    }
}

impl ExternalSigner for HsmSigner {
    fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    fn sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        if !self.available {
            return Err(SignerError::SignerUnavailable {
                signer_type: "HsmSigner",
                reason: format!(
                    "HSM slot {} label '{}' is offline",
                    self.slot, self.key_label
                ),
            });
        }

        if let Some(client) = &self.signing_client {
            client(message)
        } else {
            Err(SignerError::SignerUnavailable {
                signer_type: "HsmSigner",
                reason: format!(
                    "HSM hardware module for slot {} label '{}' is not initialized",
                    self.slot, self.key_label
                ),
            })
        }
    }

    fn sign_transaction(&self, tx: &mut Transaction) -> Result<(), SignerError> {
        if !self.available {
            return Err(SignerError::SignerUnavailable {
                signer_type: "HsmSigner",
                reason: format!(
                    "HSM slot {} label '{}' is offline",
                    self.slot, self.key_label
                ),
            });
        }

        if tx.message.recent_blockhash == solana_sdk::hash::Hash::default() {
            return Err(SignerError::InvalidTransactionState(
                "Transaction blockhash must be set before signing".to_string(),
            ));
        }

        let message_data = tx.message_data();
        let sig = self.sign_message(&message_data)?;

        let signer_index = tx
            .message
            .account_keys
            .iter()
            .position(|k| *k == self.pubkey)
            .ok_or_else(|| {
                SignerError::SigningFailed(format!(
                    "Signer pubkey {} not found in transaction accounts",
                    self.pubkey
                ))
            })?;

        if tx.signatures.len() <= signer_index {
            tx.signatures.resize(
                tx.message.header.num_required_signatures as usize,
                Signature::default(),
            );
        }
        tx.signatures[signer_index] = sig;

        Ok(())
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn signer_type(&self) -> &'static str {
        "HsmSigner"
    }

    fn is_production_allowed(&self) -> bool {
        true
    }
}

// ============================================================================
// 7. PRODUCTION SIGNER LOADER & VERIFIER
// ============================================================================

/// Loads and verifies an authentic production cryptographic signer based on explicit application configuration.
///
/// # Security Guarantees:
/// 1. Missing signer configuration FAILS CLOSED.
/// 2. Only authentic production implementations (real KeypairSigner, real KMS, real HSM) are permitted.
/// 3. DevTestSigner, simulated enclave keys, fake signatures, and deterministic dev secrets are strictly rejected.
/// 4. Loaded signer's public key MUST match the configured expected execution authority; mismatches are rejected.
/// 5. Never logs private key material or serialized secret keys.
pub fn load_production_signer(
    config: &crate::config::Config,
) -> Result<Arc<dyn ExternalSigner>, SignerError> {
    let backend = config.signer_backend.trim().to_lowercase();
    if backend.is_empty() {
        return Err(SignerError::MissingConfiguration(
            "Production signer backend not configured; set SIGNER_BACKEND explicitly (fail closed)"
                .to_string(),
        ));
    }

    let signer: Arc<dyn ExternalSigner> = match backend.as_str() {
        "keypair" | "localkeypair" | "file" => {
            let path = config.execution_signer_path.trim();
            if path.is_empty() {
                return Err(SignerError::MissingConfiguration(
                    "Signer backend 'keypair' selected but EXECUTION_SIGNER_PATH is empty"
                        .to_string(),
                ));
            }
            let keypair_signer = KeypairSigner::load_from_path(path)?;
            if !keypair_signer.is_production_allowed() {
                return Err(SignerError::ProductionFallbackProhibited(
                    "Deterministic development secret or test key cannot be used in production"
                        .to_string(),
                ));
            }
            Arc::new(keypair_signer)
        }
        "kms" | "awskms" | "gcpkms" => {
            let key_id = config.kms_key_id.as_deref().unwrap_or("").trim();
            if key_id.is_empty() {
                return Err(SignerError::MissingConfiguration(
                    "Signer backend 'kms' selected but KMS_KEY_ID is missing".to_string(),
                ));
            }
            let expected_auth = config
                .expected_execution_authority
                .as_deref()
                .unwrap_or("")
                .trim();
            if expected_auth.is_empty() {
                return Err(SignerError::MissingConfiguration(
                    "EXPECTED_EXECUTION_AUTHORITY is required when initializing KMS signer"
                        .to_string(),
                ));
            }
            let pubkey = solana_sdk::pubkey::Pubkey::from_str(expected_auth).map_err(|e| {
                SignerError::InvalidConfiguration(format!(
                    "Invalid expected authority pubkey: {}",
                    e
                ))
            })?;
            let kms_signer = KmsSigner::new(key_id, config.kms_endpoint.clone(), pubkey)?;
            Arc::new(kms_signer)
        }
        "hsm" | "pkcs11" | "cloudhsm" => {
            let key_label = config.hsm_key_label.as_deref().unwrap_or("").trim();
            if key_label.is_empty() {
                return Err(SignerError::MissingConfiguration(
                    "Signer backend 'hsm' selected but HSM_KEY_LABEL is missing".to_string(),
                ));
            }
            let expected_auth = config
                .expected_execution_authority
                .as_deref()
                .unwrap_or("")
                .trim();
            if expected_auth.is_empty() {
                return Err(SignerError::MissingConfiguration(
                    "EXPECTED_EXECUTION_AUTHORITY is required when initializing HSM signer"
                        .to_string(),
                ));
            }
            let pubkey = solana_sdk::pubkey::Pubkey::from_str(expected_auth).map_err(|e| {
                SignerError::InvalidConfiguration(format!(
                    "Invalid expected authority pubkey: {}",
                    e
                ))
            })?;
            let hsm_signer = HsmSigner::new(config.hsm_slot.unwrap_or(0), key_label, pubkey)?;
            Arc::new(hsm_signer)
        }
        "devtest" | "dev" | "test" | "ephemeral" | "mock" => {
            return Err(SignerError::ProductionFallbackProhibited(format!(
                "Signer backend '{}' is a development/test signer and is strictly prohibited in production",
                config.signer_backend
            )));
        }
        other => {
            return Err(SignerError::InvalidConfiguration(format!(
                "Unrecognized signer backend '{}'. Allowed production backends: 'keypair', 'kms', 'hsm'",
                other
            )));
        }
    };

    // Fail closed if signer is unavailable
    if !signer.is_available() {
        return Err(SignerError::SignerUnavailable {
            signer_type: signer.signer_type(),
            reason: "Signer hardware/service failed initial availability check or is offline"
                .to_string(),
        });
    }

    // Verify public key against configured expected execution authority
    if let Some(expected_auth_str) = &config.expected_execution_authority {
        let expected_auth_str = expected_auth_str.trim();
        if !expected_auth_str.is_empty() {
            let expected_pubkey =
                solana_sdk::pubkey::Pubkey::from_str(expected_auth_str).map_err(|e| {
                    SignerError::InvalidConfiguration(format!(
                        "Invalid expected execution authority public key '{}': {}",
                        expected_auth_str, e
                    ))
                })?;
            if signer.pubkey() != expected_pubkey {
                return Err(SignerError::AuthorityMismatch {
                    expected: expected_pubkey.to_string(),
                    actual: signer.pubkey_string(),
                });
            }
        }
    } else if config.environment == crate::config::Environment::Mainnet {
        return Err(SignerError::MissingConfiguration(
            "EXPECTED_EXECUTION_AUTHORITY must be explicitly configured in Mainnet production"
                .to_string(),
        ));
    }

    info!(
        signer_type = signer.signer_type(),
        authority = %signer.pubkey(),
        "Production cryptographic signer loaded and verified successfully"
    );

    Ok(signer)
}

// Helper to expand "~/" in paths
fn shellexpand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    path.to_string()
}
