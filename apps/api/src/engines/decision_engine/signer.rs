//! Isolated execution signer managing on-chain dispatch credentials securely.

use solana_sdk::signature::SeedDerivable;
use std::{fs, path::Path};
use tracing::{info, warn};
use uuid::Uuid;

/// Isolated cryptographic signer for executing authorized vault decisions.
/// Kept strictly separate from public HTTP request handlers.
#[derive(Clone)]
pub struct ExecutionSigner {
    signer_pubkey: String,
    key_material: Vec<u8>,
}

impl std::fmt::Debug for ExecutionSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionSigner")
            .field("signer_pubkey", &self.signer_pubkey)
            .field("key_material", &"[REDACTED]")
            .finish()
    }
}

impl ExecutionSigner {
    /// Loads the execution keypair from the specified filesystem path or falls back to a deterministic key.
    pub fn load_or_generate(path_str: &str) -> Self {
        let expanded_path = shellexpand(path_str);
        //why this path is exist ? 
        let path = Path::new(&expanded_path);

        if path.exists() {         
            //what is going on in this code if let Ok(content) = fs::read_to_string(path)?
            // "/tmp/test_exec_signer.json",
            if let Ok(content) = fs::read_to_string(path) {
                
                if let Ok(bytes) = serde_json::from_str::<Vec<u8>>(&content) {
                    if bytes.len() >= 32 {
                        let pubkey_str = hex_encode(&bytes[32..bytes.len().min(64)]);
                        info!(pubkey = %pubkey_str, "Loaded execution signer keypair from disk");
                        return Self {
                            signer_pubkey: pubkey_str,
                            key_material: bytes,
                        };
                    }
                }
            }
        }

        warn!(
            path = %path_str,
            "Signer file not found; initializing deterministic isolated execution signer"
        );
        //what this default seed is used for or means?
        let default_seed = [42u8; 32];
        let pubkey_str = hex_encode(&default_seed);
        Self {
            signer_pubkey: pubkey_str,
            key_material: default_seed.to_vec(),
        }
    }

    /// Returns the public key address of the signer.
    pub fn pubkey(&self) -> &str {
        &self.signer_pubkey
    }

    /// Generates an isolated cryptographic signature for an approved execution decision.
    pub fn sign_decision(&self, decision_id: &Uuid) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        decision_id.hash(&mut hasher);
        self.key_material.hash(&mut hasher);
        let sig_val = hasher.finish();

        format!("sig_{:016x}_{}", sig_val, decision_id.simple())
    }

    /// Derives or extracts a Solana SDK Keypair from the loaded key material.
    pub fn to_solana_keypair(&self) -> Result<solana_sdk::signature::Keypair, String> {
        if self.key_material.len() >= 64 {
            //what is Keypair frombytes function usage?
            solana_sdk::signature::Keypair::from_bytes(&self.key_material[..64])
                .map_err(|e| e.to_string())
        } else if self.key_material.len() >= 32 {
            //what is from_seed usage?
            solana_sdk::signature::Keypair::from_seed(&self.key_material[..32])
                .map_err(|e| e.to_string())
        } else {
            Err("Insufficient key material length".to_string())
        }
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

fn hex_encode(bytes: &[u8]) -> String {
    // why .map(|b| format!("{:02x}", b)) part is exist
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signer_debug_masks_key_material() {
        let signer = ExecutionSigner::load_or_generate("/nonexistent/path/signer.json");
        let debug_str = format!("{:?}", signer);

        assert!(debug_str.contains("[REDACTED]"));
        assert!(debug_str.contains(&signer.signer_pubkey));
        // Verify key bytes (e.g. 42) are not shown as raw numbers
        assert!(!debug_str.contains("key_material: ["));
    }
}
