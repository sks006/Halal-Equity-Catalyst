use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::pubkey::Pubkey;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsNotification {
    pub signature: String,
    pub err: Option<Value>,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountNotification {
    pub lamports: u64,
    pub owner: String,
    pub data: Vec<u8>,
    pub executable: bool,
    pub rent_epoch: u64,
}

pub struct SolanaWebSocketClient {
    ws_url: String,
    request_id: AtomicU64,
}

impl SolanaWebSocketClient {
    pub fn new(ws_url: impl Into<String>) -> Self {
        Self {
            ws_url: ws_url.into(),
            request_id: AtomicU64::new(1),
        }
    }

    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    /// Subscribes to transaction logs mentioning the given program ID.
    /// Emits `LogsNotification` whenever a relevant transaction is processed on-chain.
    pub async fn logs_subscribe(
        &self,
        program_id: &Pubkey,
    ) -> Result<mpsc::Receiver<LogsNotification>, crate::SolanaError> {
        let (ws_stream, _) = connect_async(&self.ws_url)
            .await
            .map_err(|e| crate::SolanaError::WebSocketTransport(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();
        let id = self.request_id.fetch_add(1, Ordering::Relaxed);

        let subscribe_msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "logsSubscribe",
            "params": [
                {
                    "mentions": [program_id.to_string()]
                },
                {
                    "commitment": "confirmed"
                }
            ]
        });

        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await
            .map_err(|e| crate::SolanaError::WebSocketTransport(e.to_string()))?;

        let (tx, rx) = mpsc::channel::<LogsNotification>(100);

        tokio::spawn(async move {
            info!("Solana logs subscription established");
            while let Some(msg_res) = read.next().await {
                match msg_res {
                    Ok(Message::Text(text)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            if let Some(params) = v.get("params") {
                                if let Some(result) = params.get("result") {
                                    if let Some(val) = result.get("value") {
                                        let signature = val
                                            .get("signature")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let err = val.get("err").cloned().filter(|e| !e.is_null());
                                        let logs = val
                                            .get("logs")
                                            .and_then(|l| l.as_array())
                                            .map(|arr| {
                                                arr.iter()
                                                    .filter_map(|item| item.as_str())
                                                    .map(|s| s.to_string())
                                                    .collect::<Vec<String>>()
                                            })
                                            .unwrap_or_default();

                                        let notification = LogsNotification {
                                            signature,
                                            err,
                                            logs,
                                        };

                                        if tx.send(notification).await.is_err() {
                                            debug!(
                                                "Logs receiver dropped, closing WebSocket listener"
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(p)) => {
                        let _ = write.send(Message::Pong(p)).await;
                    }
                    Ok(Message::Close(_)) => {
                        info!("Solana WebSocket closed by remote server");
                        break;
                    }
                    Err(e) => {
                        error!(error = %e, "Solana WebSocket read error");
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(rx)
    }

    /// Subscribes to account state mutations for a specific account.
    pub async fn account_subscribe(
        &self,
        pubkey: &Pubkey,
    ) -> Result<mpsc::Receiver<AccountNotification>, crate::SolanaError> {
        let (ws_stream, _) = connect_async(&self.ws_url)
            .await
            .map_err(|e| crate::SolanaError::WebSocketTransport(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();
        let id = self.request_id.fetch_add(1, Ordering::Relaxed);

        let subscribe_msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "accountSubscribe",
            "params": [
                pubkey.to_string(),
                {
                    "encoding": "base64",
                    "commitment": "confirmed"
                }
            ]
        });

        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await
            .map_err(|e| crate::SolanaError::WebSocketTransport(e.to_string()))?;

        let (tx, rx) = mpsc::channel::<AccountNotification>(100);
        let sub_pubkey = *pubkey;

        tokio::spawn(async move {
            info!(pubkey = %sub_pubkey, "Solana account subscription established");
            while let Some(msg_res) = read.next().await {
                match msg_res {
                    Ok(Message::Text(text)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            if let Some(params) = v.get("params") {
                                if let Some(result) = params.get("result") {
                                    if let Some(val) = result.get("value") {
                                        let lamports = val
                                            .get("lamports")
                                            .and_then(|l| l.as_u64())
                                            .unwrap_or(0);
                                        let owner = val
                                            .get("owner")
                                            .and_then(|o| o.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let executable = val
                                            .get("executable")
                                            .and_then(|e| e.as_bool())
                                            .unwrap_or(false);
                                        let rent_epoch = val
                                            .get("rentEpoch")
                                            .and_then(|r| r.as_u64())
                                            .unwrap_or(0);
                                        let data = if let Some(arr) =
                                            val.get("data").and_then(|d| d.as_array())
                                        {
                                            if let Some(b64) = arr.first().and_then(|s| s.as_str())
                                            {
                                                base64::engine::general_purpose::STANDARD
                                                    .decode(b64)
                                                    .unwrap_or_default()
                                            } else {
                                                Vec::new()
                                            }
                                        } else {
                                            Vec::new()
                                        };

                                        let notification = AccountNotification {
                                            lamports,
                                            owner,
                                            data,
                                            executable,
                                            rent_epoch,
                                        };

                                        if tx.send(notification).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(p)) => {
                        let _ = write.send(Message::Pong(p)).await;
                    }
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        warn!(error = %e, "Solana account WebSocket read error");
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(rx)
    }
}
