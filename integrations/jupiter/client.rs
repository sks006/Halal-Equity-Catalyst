//! Jupiter v6 HTTP client.

use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
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
    supported_mints: Arc<RwLock<Option<HashSet<String>>>>,
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
            supported_mints: Arc::new(RwLock::new(None)),
        }
    }

    /// Creates a client operating in mock mode for testing without network requests.
    pub fn new_mock() -> Self {
        let client = Self::new("https://mock-quote-api.jup.ag/v6");
        *client.mock_mode.write().unwrap() = true;
        client
    }

    /// Returns the configured base URL for the Jupiter provider.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns whether the client is currently in mock mode.
    pub fn is_mock_mode(&self) -> bool {
        *self.mock_mode.read().unwrap()
    }

    /// Enables or disables mock mode.
    pub fn set_mock_mode(&self, enabled: bool) {
        *self.mock_mode.write().unwrap() = enabled;
    }

    /// Sets the optional permitted/supported mint whitelist.
    pub fn set_supported_mints(&self, mints: impl IntoIterator<Item = impl Into<String>>) {
        let set: HashSet<String> = mints.into_iter().map(Into::into).collect();
        *self.supported_mints.write().unwrap() = Some(set);
    }

    /// Clears any configured supported mint whitelist.
    pub fn clear_supported_mints(&self) {
        *self.supported_mints.write().unwrap() = None;
    }

    /// Checks if a mint is permitted by the configured supported mint whitelist.
    pub fn is_mint_supported(&self, mint: &str) -> bool {
        let guard = self.supported_mints.read().unwrap();
        match *guard {
            Some(ref set) => set.contains(mint),
            None => true,
        }
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
        // 1. Validate input amount > 0
        if request.amount == 0 {
            return Err(JupiterError::InvalidQuote(
                "Input amount must be greater than zero".to_string(),
            ));
        }

        // 2. Validate mint addresses
        Pubkey::from_str(&request.input_mint).map_err(|e| {
            JupiterError::InvalidQuote(format!(
                "Invalid input mint '{}': {}",
                request.input_mint, e
            ))
        })?;
        Pubkey::from_str(&request.output_mint).map_err(|e| {
            JupiterError::InvalidQuote(format!(
                "Invalid output mint '{}': {}",
                request.output_mint, e
            ))
        })?;

        if request.input_mint == request.output_mint {
            return Err(JupiterError::UnsupportedPair {
                input_mint: request.input_mint.clone(),
                output_mint: request.output_mint.clone(),
            });
        }

        if !self.is_mint_supported(&request.input_mint)
            || !self.is_mint_supported(&request.output_mint)
        {
            return Err(JupiterError::UnsupportedPair {
                input_mint: request.input_mint.clone(),
                output_mint: request.output_mint.clone(),
            });
        }

        // 3. Mock mode handling
        if *self.mock_mode.read().unwrap() {
            let mocks = self.mock_quotes.read().unwrap();
            let key = (request.input_mint.clone(), request.output_mint.clone());
            if let Some(quote) = mocks.get(&key) {
                let out_amount: u64 = quote.out_amount.parse().unwrap_or(0);
                if out_amount == 0 {
                    return Err(JupiterError::InvalidQuote(
                        "Expected output amount must be greater than zero".to_string(),
                    ));
                }
                if quote.route_plan.is_empty() {
                    return Err(JupiterError::NoRoute {
                        input_mint: request.input_mint.clone(),
                        output_mint: request.output_mint.clone(),
                    });
                }
                return Ok(quote.clone());
            }
            return Err(JupiterError::InvalidQuote(format!(
                "No mock quote found for {} -> {}",
                request.input_mint, request.output_mint
            )));
        }

        // 4. Production query
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

        debug!(url = %url, "Querying real Jupiter quote endpoint");

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

        // 5. Validate provider response values
        let out_amount: u64 = quote
            .out_amount
            .parse()
            .map_err(|e| JupiterError::InvalidQuote(format!("Invalid out_amount: {}", e)))?;
        if out_amount == 0 {
            return Err(JupiterError::InvalidQuote(
                "Expected output amount must be greater than zero".to_string(),
            ));
        }

        if quote.route_plan.is_empty() {
            return Err(JupiterError::NoRoute {
                input_mint: request.input_mint.clone(),
                output_mint: request.output_mint.clone(),
            });
        }

        Ok(quote)
    }

    /// Assembles a serialized swap transaction from `/v6/swap`.
    ///
    /// Never returns a dummy serialized transaction.
    /// Provider failure returns an error.
    /// Does not sign transactions. Keeps construction, signing, and submission strictly decoupled.
    pub async fn build_swap(&self, request: &SwapRequest) -> Result<SwapResponse, JupiterError> {
        // 1. Validate user_public_key (execution authority)
        let clean_pubkey = request.user_public_key.trim();
        if clean_pubkey.is_empty() {
            return Err(JupiterError::InvalidTransaction(
                "Execution authority public key cannot be empty".to_string(),
            ));
        }
        Pubkey::from_str(clean_pubkey).map_err(|e| {
            JupiterError::InvalidTransaction(format!(
                "Invalid execution authority public key '{}': {}",
                clean_pubkey, e
            ))
        })?;

        // 2. Validate quote response payload
        let in_amount: u64 =
            request.quote_response.in_amount.parse().map_err(|e| {
                JupiterError::InvalidQuote(format!("Invalid quote in_amount: {}", e))
            })?;
        let out_amount: u64 =
            request.quote_response.out_amount.parse().map_err(|e| {
                JupiterError::InvalidQuote(format!("Invalid quote out_amount: {}", e))
            })?;

        if in_amount == 0 || out_amount == 0 {
            return Err(JupiterError::InvalidQuote(
                "Swap quote must have positive input and output amounts".to_string(),
            ));
        }

        if request.quote_response.route_plan.is_empty() {
            return Err(JupiterError::NoRoute {
                input_mint: request.quote_response.input_mint.clone(),
                output_mint: request.quote_response.output_mint.clone(),
            });
        }

        // 3. Mock mode handling
        if *self.mock_mode.read().unwrap() {
            let mocks = self.mock_swaps.read().unwrap();
            if let Some(swap) = mocks.get(&request.quote_response.out_amount) {
                if swap.swap_transaction.trim().is_empty() || swap.is_dummy() {
                    return Err(JupiterError::InvalidTransaction(
                        "Mock swap response contains empty or dummy transaction payload"
                            .to_string(),
                    ));
                }
                return Ok(swap.clone());
            }
            // CRITICAL: NEVER return a dummy serialized transaction!
            // Fail closed if no mock is configured.
            return Err(JupiterError::InvalidTransaction(format!(
                "No mock swap response configured for out_amount: {}",
                request.quote_response.out_amount
            )));
        }

        // 4. Production query to real DEX provider
        let url = format!("{}/swap", self.base_url);
        debug!(
            url = %url,
            user = %clean_pubkey,
            "Requesting real Jupiter swap transaction from provider"
        );

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
            warn!(status = %status, body = %body, "Jupiter swap build failed from provider");
            return Err(JupiterError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        let swap: SwapResponse = resp
            .json()
            .await
            .map_err(|e| JupiterError::Json(e.to_string()))?;

        // 5. Verify provider returned a real non-empty, non-dummy transaction
        if swap.swap_transaction.trim().is_empty() || swap.is_dummy() {
            return Err(JupiterError::InvalidTransaction(
                "Provider returned an empty or dummy swap transaction payload".to_string(),
            ));
        }

        Ok(swap)
    }
}
