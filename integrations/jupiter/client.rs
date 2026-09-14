//! Jupiter v6 HTTP client.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, warn};

use crate::types::{JupiterError, QuoteRequest, QuoteResponse, SwapRequest, SwapResponse};

/// Client for interacting with Jupiter's v6 Quote and Swap HTTP API.
#[derive(Clone)]
pub struct JupiterClient {
    base_url: String,
    http_client: reqwest::Client,
    mock_mode: Arc<RwLock<bool>>,
    mock_quotes: Arc<RwLock<HashMap<(String, String), QuoteResponse>>>,
    mock_swaps: Arc<RwLock<HashMap<String, SwapResponse>>>,
}

impl JupiterClient {
    /// Creates a new JupiterClient connected to the specified API base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            mock_mode: Arc::new(RwLock::new(false)),
            mock_quotes: Arc::new(RwLock::new(HashMap::new())),
            mock_swaps: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a client operating in mock mode for testing without network requests.
    pub fn new_mock() -> Self {
        let client = Self::new("https://mock-quote-api.jup.ag/v6");
        *client.mock_mode.write().unwrap() = true;
        client
    }

    /// Enables or disables mock mode.
    pub fn set_mock_mode(&self, enabled: bool) {
        *self.mock_mode.write().unwrap() = enabled;
    }

    /// Injects a mock quote response for a specific token pair.
    pub fn set_mock_quote(&self, input_mint: &str, output_mint: &str, quote: QuoteResponse) {
        let mut mocks = self.mock_quotes.write().unwrap();
        mocks.insert((input_mint.to_string(), output_mint.to_string()), quote);
    }

    /// Injects a mock swap response.
    pub fn set_mock_swap(&self, quote_out_amount: &str, swap: SwapResponse) {
        let mut mocks = self.mock_swaps.write().unwrap();
        mocks.insert(quote_out_amount.to_string(), swap);
    }

    /// Queries a quote from Jupiter `/v6/quote`.
    pub async fn get_quote(&self, request: &QuoteRequest) -> Result<QuoteResponse, JupiterError> {
        if *self.mock_mode.read().unwrap() {
            let mocks = self.mock_quotes.read().unwrap();
            let key = (request.input_mint.clone(), request.output_mint.clone());
            if let Some(quote) = mocks.get(&key) {
                return Ok(quote.clone());
            }
            return Err(JupiterError::InvalidQuote(format!(
                "No mock quote found for {} -> {}",
                request.input_mint, request.output_mint
            )));
        }

        let mut url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}",
            self.base_url, request.input_mint, request.output_mint, request.amount
        );

        if let Some(slippage) = request.slippage_bps {
            url.push_str(&format!("&slippageBps={}", slippage));
        }

        if let Some(ref mode) = request.swap_mode {
            url.push_str(&format!("&swapMode={}", mode));
        }

        if let Some(direct) = request.only_direct_routes {
            url.push_str(&format!("&onlyDirectRoutes={}", direct));
        }

        debug!(url = %url, "Querying Jupiter quote");

        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| JupiterError::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            warn!(status = %status, body = %body, "Jupiter quote query failed");
            return Err(JupiterError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        let quote: QuoteResponse = resp
            .json()
            .await
            .map_err(|e| JupiterError::Json(e.to_string()))?;

        Ok(quote)
    }

    /// Assembles a serialized swap transaction from `/v6/swap`.
    pub async fn build_swap(&self, request: &SwapRequest) -> Result<SwapResponse, JupiterError> {
        if *self.mock_mode.read().unwrap() {
            let mocks = self.mock_swaps.read().unwrap();
            if let Some(swap) = mocks.get(&request.quote_response.out_amount) {
                return Ok(swap.clone());
            }
            // Return dummy fallback swap if not explicitly mocked
            return Ok(SwapResponse {
                swap_transaction: "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDBg==".to_string(),
                last_valid_block_height: 250_000_000,
                prioritization_fee_lamports: request.prioritization_fee_lamports,
            });
        }

        let url = format!("{}/swap", self.base_url);
        debug!(url = %url, user = %request.user_public_key, "Requesting Jupiter swap transaction");

        let resp = self
            .http_client
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(|e| JupiterError::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            warn!(status = %status, body = %body, "Jupiter swap build failed");
            return Err(JupiterError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        let swap: SwapResponse = resp
            .json()
            .await
            .map_err(|e| JupiterError::Json(e.to_string()))?;

        Ok(swap)
    }
}
