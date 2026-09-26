use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::{hash::Hash, pubkey::Pubkey, signature::Signature};
use std::{
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tracing::{debug, info, instrument, warn};

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

/// Solana cluster transaction confirmation state (Phase P7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionConfirmationStatus {
    /// Confirmed on-chain without error.
    ConfirmedSuccess {
        slot: u64,
        confirmations: Option<usize>,
    },
    /// Transaction landed on-chain but execution failed / reverted with an error.
    ConfirmedFailure { slot: u64, error: String },
    /// Block height exceeded transaction lastValidBlockHeight before confirmation landed.
    ExpiredTransaction {
        last_valid_block_height: u64,
        current_block_height: u64,
    },
    /// Polling duration exceeded timeout before cluster confirmed or proved expiration.
    RpcTimeout { timeout_secs: u64 },
    /// RPC returned an unknown or indeterminate status.
    UnknownStatus { reason: String },
}

impl TransactionConfirmationStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::ConfirmedSuccess { .. })
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, Self::ConfirmedFailure { .. })
    }

    pub fn is_expired(&self) -> bool {
        matches!(self, Self::ExpiredTransaction { .. })
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::RpcTimeout { .. })
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::UnknownStatus { .. })
    }

    pub fn status_code(&self) -> &'static str {
        match self {
            Self::ConfirmedSuccess { .. } => "CONFIRMED_SUCCESS",
            Self::ConfirmedFailure { .. } => "CONFIRMED_FAILURE",
            Self::ExpiredTransaction { .. } => "EXPIRED_TRANSACTION",
            Self::RpcTimeout { .. } => "RPC_TIMEOUT",
            Self::UnknownStatus { .. } => "UNKNOWN_STATUS",
        }
    }
}

pub struct SolanaRpcClient {
    client: reqwest::Client,
    rpc_url: String,
    fallback_urls: Vec<String>,
    commitment: String,
    request_id: AtomicU64,
    timeout: Duration,
    max_retries: u32,
}

impl SolanaRpcClient {
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self::new_with_fallbacks(rpc_url, Vec::new())
    }

    pub fn new_with_fallbacks(primary_url: impl Into<String>, fallback_urls: Vec<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            rpc_url: primary_url.into(),
            fallback_urls,
            commitment: "confirmed".to_string(),
            request_id: AtomicU64::new(1),
            timeout: Duration::from_secs(30),
            max_retries: 2,
        }
    }

    pub fn with_fallback(mut self, fallback_url: impl Into<String>) -> Self {
        let url = fallback_url.into();
        if !url.is_empty() && url != self.rpc_url && !self.fallback_urls.contains(&url) {
            self.fallback_urls.push(url);
        }
        self
    }

    pub fn with_fallbacks(
        mut self,
        fallback_urls: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        for fb in fallback_urls {
            self = self.with_fallback(fb);
        }
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_commitment(mut self, commitment: &str) -> Self {
        self.commitment = commitment.to_string();
        self
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    pub fn fallback_urls(&self) -> &[String] {
        &self.fallback_urls
    }

    pub fn all_endpoints(&self) -> Vec<String> {
        let mut list = Vec::with_capacity(1 + self.fallback_urls.len());
        list.push(self.rpc_url.clone());
        for fb in &self.fallback_urls {
            if fb != &self.rpc_url && !list.contains(fb) {
                list.push(fb.clone());
            }
        }
        list
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    pub fn commitment(&self) -> &str {
        &self.commitment
    }

    async fn send_rpc_request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, crate::SolanaError> {
        let endpoints = self.all_endpoints();
        let total_rounds = self.max_retries.saturating_add(1);
        let mut last_error: Option<crate::SolanaError> = None;

        for round in 0..total_rounds {
            if round > 0 {
                let backoff_ms = (150 * round as u64).min(2000);
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }

            for endpoint in &endpoints {
                let id = self.request_id.fetch_add(1, Ordering::Relaxed);
                let payload = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": method,
                    "params": params,
                });

                debug!(
                    endpoint = %endpoint,
                    method,
                    id,
                    round,
                    "Sending Solana RPC request"
                );

                let send_res = self
                    .client
                    .post(endpoint)
                    .timeout(self.timeout)
                    .json(&payload)
                    .send()
                    .await;

                let resp = match send_res {
                    Ok(resp) => resp,
                    Err(e) => {
                        warn!(
                            endpoint = %endpoint,
                            error = %e,
                            "Solana RPC transport error, attempting next endpoint"
                        );
                        last_error = Some(crate::SolanaError::RpcTransport(format!(
                            "[{}] {}",
                            endpoint, e
                        )));
                        continue;
                    }
                };

                let status = resp.status();
                if !status.is_success() {
                    let err_msg = format!("HTTP error: {}", status);
                    let is_transient_http = status == reqwest::StatusCode::TOO_MANY_REQUESTS
                        || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
                        || status == reqwest::StatusCode::BAD_GATEWAY
                        || status == reqwest::StatusCode::GATEWAY_TIMEOUT
                        || status == reqwest::StatusCode::INTERNAL_SERVER_ERROR;

                    if is_transient_http {
                        warn!(
                            endpoint = %endpoint,
                            status = %status,
                            "Transient Solana RPC HTTP status (429/5xx), attempting retry"
                        );
                        last_error = Some(crate::SolanaError::RpcError {
                            code: status.as_u16() as i64,
                            message: format!("[{}] {}", endpoint, err_msg),
                        });
                        continue;
                    } else {
                        // Non-transient client error (400, 401, 403, 404): fail immediately
                        return Err(crate::SolanaError::RpcError {
                            code: status.as_u16() as i64,
                            message: format!(
                                "[{}] Permanent HTTP client error: {}",
                                endpoint, err_msg
                            ),
                        });
                    }
                }

                let json_resp: Value = match resp.json().await {
                    Ok(val) => val,
                    Err(e) => {
                        warn!(
                            endpoint = %endpoint,
                            error = %e,
                            "Failed to parse Solana RPC JSON response, attempting next endpoint"
                        );
                        last_error = Some(crate::SolanaError::RpcTransport(format!(
                            "[{}] Failed to parse JSON: {}",
                            endpoint, e
                        )));
                        continue;
                    }
                };

                if let Some(err) = json_resp.get("error") {
                    let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                    let msg = err
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown RPC error")
                        .to_string();

                    let is_retryable_node_issue = code == -32005
                        || code == -32429
                        || msg.to_lowercase().contains("rate limit")
                        || msg.to_lowercase().contains("behind")
                        || msg.to_lowercase().contains("exceeded");

                    if is_retryable_node_issue && endpoints.len() > 1 {
                        warn!(
                            endpoint = %endpoint,
                            code,
                            msg = %msg,
                            "Solana RPC rate-limited or node behind, attempting next endpoint"
                        );
                        last_error = Some(crate::SolanaError::RpcError {
                            code,
                            message: format!("[{}] {}", endpoint, msg),
                        });
                        continue;
                    } else {
                        return Err(crate::SolanaError::RpcError { code, message: msg });
                    }
                }

                if let Some(result) = json_resp.get("result").cloned() {
                    return Ok(result);
                } else {
                    warn!(
                        endpoint = %endpoint,
                        "Missing 'result' in Solana RPC response, attempting next endpoint"
                    );
                    last_error = Some(crate::SolanaError::RpcError {
                        code: -1,
                        message: format!("[{}] Missing 'result' in RPC response", endpoint),
                    });
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            crate::SolanaError::RpcTransport(
                "All Solana RPC endpoints exhausted without response".to_string(),
            )
        }))
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

    #[instrument(skip(self))]
    pub async fn get_block_height(&self) -> Result<u64, crate::SolanaError> {
        let params = json!([{
            "commitment": self.commitment,
        }]);

        let result = self.send_rpc_request("getBlockHeight", params).await?;
        result.as_u64().ok_or_else(|| crate::SolanaError::RpcError {
            code: -1,
            message: "Missing or invalid getBlockHeight in RPC response".to_string(),
        })
    }

    #[instrument(skip(self), fields(sig = %sig))]
    pub async fn get_transaction(
        &self,
        sig: &Signature,
    ) -> Result<Option<Value>, crate::SolanaError> {
        let params = json!([
            sig.to_string(),
            {
                "encoding": "json",
                "commitment": self.commitment,
                "maxSupportedTransactionVersion": 0
            }
        ]);

        let result = self.send_rpc_request("getTransaction", params).await?;
        if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    /// Tracks transaction confirmation on the Solana cluster (Phase P7).
    ///
    /// Distinguishes between:
    /// - ConfirmedSuccess: On-chain confirmed with err == null
    /// - ConfirmedFailure: On-chain confirmed with err != null (reverted)
    /// - ExpiredTransaction: Cluster block height exceeded last_valid_block_height without transaction landing
    /// - RpcTimeout: Polling timeout reached before resolution
    /// - UnknownStatus: Status indeterminate
    #[instrument(skip(self), fields(sig = %sig))]
    pub async fn track_transaction_confirmation(
        &self,
        sig: &Signature,
        last_valid_block_height: Option<u64>,
        timeout: Duration,
    ) -> Result<TransactionConfirmationStatus, crate::SolanaError> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(500);

        while start.elapsed() < timeout {
            // 1. Check signature status
            match self.get_signature_status(sig).await {
                Ok(Some(status)) => {
                    if let Some(ref err) = status.err {
                        warn!(
                            sig = %sig,
                            slot = status.slot,
                            error = %err,
                            "Transaction landed but confirmed failure on Solana cluster"
                        );
                        return Ok(TransactionConfirmationStatus::ConfirmedFailure {
                            slot: status.slot,
                            error: err.to_string(),
                        });
                    }

                    if let Some(ref conf) = status.confirmation_status {
                        if conf == "confirmed" || conf == "finalized" {
                            info!(
                                sig = %sig,
                                slot = status.slot,
                                status = %conf,
                                "Transaction confirmed successfully on Solana cluster"
                            );
                            return Ok(TransactionConfirmationStatus::ConfirmedSuccess {
                                slot: status.slot,
                                confirmations: status.confirmations,
                            });
                        }
                    }
                }
                Ok(None) => {
                    // Not in recent signature cache, check if block height expired
                    if let Some(last_valid) = last_valid_block_height {
                        if let Ok(current_height) = self.get_block_height().await {
                            if current_height > last_valid {
                                // Final check via getTransaction in case getSignatureStatuses dropped it
                                if let Ok(Some(tx_val)) = self.get_transaction(sig).await {
                                    let slot =
                                        tx_val.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);
                                    let meta_err = tx_val
                                        .get("meta")
                                        .and_then(|m| m.get("err"))
                                        .filter(|e| !e.is_null());
                                    if let Some(err) = meta_err {
                                        return Ok(
                                            TransactionConfirmationStatus::ConfirmedFailure {
                                                slot,
                                                error: err.to_string(),
                                            },
                                        );
                                    } else {
                                        return Ok(
                                            TransactionConfirmationStatus::ConfirmedSuccess {
                                                slot,
                                                confirmations: None,
                                            },
                                        );
                                    }
                                }
                                warn!(
                                    sig = %sig,
                                    current_height,
                                    last_valid,
                                    "Transaction expired: cluster block height exceeded last valid block height"
                                );
                                return Ok(TransactionConfirmationStatus::ExpiredTransaction {
                                    last_valid_block_height: last_valid,
                                    current_block_height: current_height,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!(sig = %sig, error = %e, "Transient RPC poll error");
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        // Final query before timing out: check get_transaction one last time
        if let Ok(Some(tx_val)) = self.get_transaction(sig).await {
            let slot = tx_val.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);
            let meta_err = tx_val
                .get("meta")
                .and_then(|m| m.get("err"))
                .filter(|e| !e.is_null());
            if let Some(err) = meta_err {
                return Ok(TransactionConfirmationStatus::ConfirmedFailure {
                    slot,
                    error: err.to_string(),
                });
            } else {
                return Ok(TransactionConfirmationStatus::ConfirmedSuccess {
                    slot,
                    confirmations: None,
                });
            }
        }

        Ok(TransactionConfirmationStatus::RpcTimeout {
            timeout_secs: timeout.as_secs(),
        })
    }
}
