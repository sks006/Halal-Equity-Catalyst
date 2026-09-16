use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::{hash::Hash, pubkey::Pubkey, signature::Signature};
use std::{
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tracing::{debug, info, instrument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub lamports: u64,
    pub owner: Pubkey,
    pub data: Vec<u8>,
    pub executable: bool,
    pub rent_epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAccountBalance {
    pub amount: u64,
    pub decimals: u8,
    pub ui_amount: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureStatus {
    pub slot: u64,
    pub confirmations: Option<usize>,
    pub err: Option<Value>,
    pub confirmation_status: Option<String>,
}

pub struct SolanaRpcClient {
    client: reqwest::Client,
    rpc_url: String,
    commitment: String,
    request_id: AtomicU64,
}

impl SolanaRpcClient {
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            rpc_url: rpc_url.into(),
            commitment: "confirmed".to_string(),
            request_id: AtomicU64::new(1),
        }
    }

    pub fn with_commitment(mut self, commitment: &str) -> Self {
        self.commitment = commitment.to_string();
        self
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    pub fn commitment(&self) -> &str {
        &self.commitment
    }

    async fn send_rpc_request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, crate::SolanaError> {
        let id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        debug!(method, id, "Sending Solana RPC request");
        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::SolanaError::RpcTransport(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(crate::SolanaError::RpcError {
                code: resp.status().as_u16() as i64,
                message: format!("HTTP error: {}", resp.status()),
            });
        }

        let json_resp: Value = resp
            .json()
            .await
            .map_err(|e| crate::SolanaError::RpcTransport(e.to_string()))?;

        if let Some(err) = json_resp.get("error") {
            let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown RPC error")
                .to_string();
            return Err(crate::SolanaError::RpcError { code, message: msg });
        }

        json_resp
            .get("result")
            .cloned()
            .ok_or_else(|| crate::SolanaError::RpcError {
                code: -1,
                message: "Missing 'result' in RPC response".to_string(),
            })
    }

    #[instrument(skip(self), fields(pubkey = %pubkey))]
    pub async fn get_account_info(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Option<AccountInfo>, crate::SolanaError> {
        let params = json!([
            pubkey.to_string(),
            {
                "encoding": "base64",
                "commitment": self.commitment,
            }
        ]);

        let result = self.send_rpc_request("getAccountInfo", params).await?;
        let value = result.get("value");
        if value.is_none() || value.unwrap().is_null() {
            return Ok(None);
        }

        let val = value.unwrap();
        let lamports = val.get("lamports").and_then(|l| l.as_u64()).unwrap_or(0);
        let owner_str = val.get("owner").and_then(|o| o.as_str()).unwrap_or("");
        let owner = Pubkey::from_str(owner_str)
            .map_err(|e| crate::SolanaError::DeserializationFailed(e.to_string()))?;
        let executable = val
            .get("executable")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);
        let rent_epoch = val.get("rentEpoch").and_then(|r| r.as_u64()).unwrap_or(0);

        let data_bytes = if let Some(data_arr) = val.get("data").and_then(|d| d.as_array()) {
            if let Some(b64_str) = data_arr.first().and_then(|s| s.as_str()) {
                BASE64
                    .decode(b64_str)
                    .map_err(|e| crate::SolanaError::DeserializationFailed(e.to_string()))?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Some(AccountInfo {
            lamports,
            owner,
            data: data_bytes,
            executable,
            rent_epoch,
        }))
    }

    #[instrument(skip(self), fields(pubkey = %pubkey))]
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, crate::SolanaError> {
        let params = json!([
            pubkey.to_string(),
            {
                "commitment": self.commitment,
            }
        ]);

        let result = self.send_rpc_request("getBalance", params).await?;
        let lamports = result
            .get("value")
            .and_then(|v| v.as_u64())
            .or_else(|| result.as_u64())
            .unwrap_or(0);
        Ok(lamports)
    }

    #[instrument(skip(self), fields(pubkey = %pubkey))]
    pub async fn get_token_account_balance(
        &self,
        pubkey: &Pubkey,
    ) -> Result<TokenAccountBalance, crate::SolanaError> {
        let params = json!([
            pubkey.to_string(),
            {
                "commitment": self.commitment,
            }
        ]);

        let result = self
            .send_rpc_request("getTokenAccountBalance", params)
            .await?;
        let val = result.get("value").unwrap_or(&result);

        let amount_str = val.get("amount").and_then(|a| a.as_str()).unwrap_or("0");
        let amount = amount_str.parse::<u64>().unwrap_or(0);
        let decimals = val.get("decimals").and_then(|d| d.as_u64()).unwrap_or(0) as u8;
        let ui_amount = val.get("uiAmount").and_then(|u| u.as_f64());

        Ok(TokenAccountBalance {
            amount,
            decimals,
            ui_amount,
        })
    }

    #[instrument(skip(self))]
    pub async fn get_latest_blockhash(&self) -> Result<(Hash, u64), crate::SolanaError> {
        let params = json!([
            {
                "commitment": self.commitment,
            }
        ]);

        let result = self.send_rpc_request("getLatestBlockhash", params).await?;
        let val = result.get("value").unwrap_or(&result);

        let hash_str = val
            .get("blockhash")
            .and_then(|b| b.as_str())
            .ok_or_else(|| crate::SolanaError::RpcError {
                code: -1,
                message: "Missing blockhash in getLatestBlockhash response".to_string(),
            })?;
        let hash = Hash::from_str(hash_str)
            .map_err(|e| crate::SolanaError::DeserializationFailed(e.to_string()))?;
        let last_valid = val
            .get("lastValidBlockHeight")
            .and_then(|h| h.as_u64())
            .unwrap_or(0);

        Ok((hash, last_valid))
    }

    #[instrument(skip(self, tx_bytes))]
    pub async fn send_transaction(&self, tx_bytes: &[u8]) -> Result<Signature, crate::SolanaError> {
        let encoded = BASE64.encode(tx_bytes);
        let params = json!([
            encoded,
            {
                "encoding": "base64",
                "preflightCommitment": self.commitment,
                "skipPreflight": false,
            }
        ]);

        let result = self.send_rpc_request("sendTransaction", params).await?;
        let sig_str = result
            .as_str()
            .ok_or_else(|| crate::SolanaError::RpcError {
                code: -1,
                message: "Missing signature in sendTransaction response".to_string(),
            })?;

        Signature::from_str(sig_str)
            .map_err(|e| crate::SolanaError::DeserializationFailed(e.to_string()))
    }

    #[instrument(skip(self, tx_bytes))]
    pub async fn simulate_transaction(&self, tx_bytes: &[u8]) -> Result<Value, crate::SolanaError> {
        let encoded = BASE64.encode(tx_bytes);
        let params = json!([
            encoded,
            {
                "encoding": "base64",
                "commitment": self.commitment,
                "sigVerify": false,
            }
        ]);

        let result = self.send_rpc_request("simulateTransaction", params).await?;
        let val = result.get("value").unwrap_or(&result);

        if let Some(err) = val.get("err").filter(|e| !e.is_null()) {
            return Err(crate::SolanaError::RpcError {
                code: -1,
                message: format!("Transaction simulation failed: {}", err),
            });
        }

        Ok(val.clone())
    }

    #[instrument(skip(self), fields(sig = %sig))]
    pub async fn get_signature_status(
        &self,
        sig: &Signature,
    ) -> Result<Option<SignatureStatus>, crate::SolanaError> {
        let params = json!([
            [sig.to_string()],
            {
                "searchTransactionHistory": true,
            }
        ]);

        let result = self
            .send_rpc_request("getSignatureStatuses", params)
            .await?;
        let values = result
            .get("value")
            .and_then(|v| v.as_array())
            .ok_or_else(|| crate::SolanaError::RpcError {
                code: -1,
                message: "Invalid getSignatureStatuses response".to_string(),
            })?;

        if let Some(first) = values.first() {
            if first.is_null() {
                return Ok(None);
            }
            let slot = first.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);
            let confirmations = first
                .get("confirmations")
                .and_then(|c| c.as_u64())
                .map(|c| c as usize);
            let err = first.get("err").cloned().filter(|e| !e.is_null());
            let confirmation_status = first
                .get("confirmationStatus")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());

            Ok(Some(SignatureStatus {
                slot,
                confirmations,
                err,
                confirmation_status,
            }))
        } else {
            Ok(None)
        }
    }

    #[instrument(skip(self), fields(sig = %sig))]
    pub async fn confirm_signature(
        &self,
        sig: &Signature,
        timeout: Duration,
    ) -> Result<SignatureStatus, crate::SolanaError> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(500);

        while start.elapsed() < timeout {
            if let Some(status) = self.get_signature_status(sig).await? {
                if let Some(ref err) = status.err {
                    return Err(crate::SolanaError::TransactionExecutionFailed {
                        signature: sig.to_string(),
                        error: err.to_string(),
                    });
                }
                if let Some(ref conf) = status.confirmation_status {
                    if conf == "confirmed" || conf == "finalized" {
                        info!(sig = %sig, status = %conf, "Transaction signature confirmed");
                        return Ok(status);
                    }
                }
            }
            tokio::time::sleep(poll_interval).await;
        }

        Err(crate::SolanaError::ConfirmationTimeout {
            signature: sig.to_string(),
            timeout_secs: timeout.as_secs(),
        })
    }
}
