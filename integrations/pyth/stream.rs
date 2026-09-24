//! Pyth Hermes Server-Sent Events (SSE) real-time streaming client.
//!
//! Connects to Pyth Hermes `/v2/updates/price/stream?parsed=true&ids[]={feed_id}&...`,
//! frames and decodes SSE events line-by-line, and yields deserialized `PythPriceUpdateEvent`
//! structures containing `ParsedPriceFeed` price models.

use futures_util::stream::Stream;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{debug, error, info, trace, warn};

use crate::subscription::normalize_feed_id;
use crate::types::{PythPriceUpdateEvent, PythStreamError};

/// Configuration for the Pyth Hermes SSE stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythStreamConfig {
    /// Base URL of the Pyth Hermes endpoint (e.g. `https://hermes.pyth.network`).
    pub base_url: String,
    /// List of Pyth feed IDs to subscribe to at runtime.
    pub feed_ids: Vec<String>,
    /// Optional API key for Hermes authentication (sent as Bearer token).
    pub api_key: Option<String>,
}

impl PythStreamConfig {
    /// Creates a new configuration with the given base URL and feed IDs.
    pub fn new(base_url: impl Into<String>, feed_ids: Vec<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            feed_ids,
            api_key: None,
        }
    }

    /// Attaches an authentication API key to the configuration.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Builds the query URL for the Hermes SSE endpoint `/v2/updates/price/stream`.
    ///
    /// Validates that `feed_ids` is not empty, normalizes feed IDs (removes `0x`, lowercase),
    /// and formats the URL as `/v2/updates/price/stream?parsed=true&ids[]={id1}&...`.
    pub fn build_url(&self) -> Result<String, PythStreamError> {
        if self.feed_ids.is_empty() {
            return Err(PythStreamError::InvalidConfig(
                "Cannot connect stream: feed_ids list is empty".to_string(),
            ));
        }

        let mut url = format!("{}/v2/updates/price/stream?parsed=true", self.base_url);
        for id in &self.feed_ids {
            let clean = normalize_feed_id(id);
            if clean.is_empty() {
                return Err(PythStreamError::InvalidConfig(
                    "Feed ID cannot be empty or solely whitespace".to_string(),
                ));
            }
            url.push_str(&format!("&ids[]={}", clean));
        }

        Ok(url)
    }

    /// Builds the HTTP headers required for SSE connection.
    pub fn build_headers(&self) -> Result<HeaderMap, PythStreamError> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));

        if let Some(ref key) = self.api_key {
            let auth_value = format!("Bearer {}", key.trim());
            let header_val = HeaderValue::from_str(&auth_value).map_err(|e| {
                PythStreamError::InvalidConfig(format!("Invalid characters in API key: {}", e))
            })?;
            headers.insert(AUTHORIZATION, header_val);
        }

        Ok(headers)
    }
}

/// Represents an intermediate parsed SSE event frame before JSON decoding.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawSseEvent {
    pub event_type: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

/// State machine parser for Server-Sent Events stream chunks.
#[derive(Debug, Default)]
pub struct SseChunkParser {
    buffer: String,
    current_event: RawSseEvent,
}

impl SseChunkParser {
    /// Creates a new chunk parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds a chunk of incoming text into the buffer and parses all completed SSE frames.
    pub fn feed_str(&mut self, chunk: &str) -> Vec<Result<RawSseEvent, PythStreamError>> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();

        while let Some(newline_pos) = self.buffer.find('\n') {
            let raw_line = self.buffer[..newline_pos].trim_end_matches('\r').to_string();
            self.buffer.drain(..=newline_pos);

            if raw_line.is_empty() {
                // An empty line signals the end of an SSE event
                if !self.current_event.data.is_empty() || self.current_event.event_type.is_some() {
                    let completed = std::mem::take(&mut self.current_event);
                    events.push(Ok(completed));
                }
            } else if raw_line.starts_with(':') {
                // Comment line (e.g. `: ping` or keep-alive heartbeat), safely ignore
                trace!(comment = %raw_line, "Ignored SSE comment/heartbeat");
            } else if let Some(stripped) = raw_line.strip_prefix("data:") {
                let data_part = stripped.strip_prefix(' ').unwrap_or(stripped);
                if !self.current_event.data.is_empty() {
                    self.current_event.data.push('\n');
                }
                self.current_event.data.push_str(data_part);
            } else if let Some(stripped) = raw_line.strip_prefix("event:") {
                let event_part = stripped.strip_prefix(' ').unwrap_or(stripped);
                self.current_event.event_type = Some(event_part.to_string());
            } else if let Some(stripped) = raw_line.strip_prefix("id:") {
                let id_part = stripped.strip_prefix(' ').unwrap_or(stripped);
                self.current_event.id = Some(id_part.to_string());
            } else if raw_line.starts_with("retry:") {
                trace!(line = %raw_line, "Received SSE retry field");
            } else {
                warn!(line = %raw_line, "Encountered unexpected SSE line format");
                events.push(Err(PythStreamError::MalformedSse(format!(
                    "Unrecognized SSE line format: '{}'",
                    raw_line
                ))));
            }
        }

        events
    }
}

/// Parses a `RawSseEvent` into a typed `PythPriceUpdateEvent`.
pub fn parse_price_update_event(raw: RawSseEvent) -> Result<PythPriceUpdateEvent, PythStreamError> {
    let event_type = raw.event_type.as_deref().unwrap_or("message");

    // Standard Pyth Hermes emits unnamed message events or "price_update"
    match event_type {
        "message" | "price_update" => {
            if raw.data.trim().is_empty() {
                return Err(PythStreamError::MalformedSse(
                    "Received price update event with empty data field".to_string(),
                ));
            }

            match serde_json::from_str::<PythPriceUpdateEvent>(&raw.data) {
                Ok(parsed_event) => Ok(parsed_event),
                Err(err) => {
                    warn!(error = %err, raw_data = %raw.data, "Failed to parse Pyth price update JSON");
                    Err(PythStreamError::MalformedJson {
                        raw: raw.data,
                        error: err.to_string(),
                    })
                }
            }
        }
        "ping" | "heartbeat" => {
            // Heartbeat event without price payload
            trace!(event = %event_type, "Received SSE ping event");
            Ok(PythPriceUpdateEvent::default())
        }
        other => {
            warn!(event = %other, data = %raw.data, "Received unknown SSE event type");
            Err(PythStreamError::UnknownEvent {
                event: other.to_string(),
                data: raw.data,
            })
        }
    }
}

/// Client for connecting to Pyth Hermes Server-Sent Events price streams.
#[derive(Debug, Clone)]
pub struct PythStreamClient {
    config: PythStreamConfig,
    http_client: reqwest::Client,
}

impl PythStreamClient {
    /// Creates a new `PythStreamClient` with the specified configuration.
    pub fn new(config: PythStreamConfig) -> Self {
        Self::new_with_http_client(
            config,
            reqwest::Client::builder()
                .build()
                .unwrap_or_default(),
        )
    }

    /// Creates a `PythStreamClient` reusing an existing `reqwest::Client`.
    pub fn new_with_http_client(config: PythStreamConfig, http_client: reqwest::Client) -> Self {
        Self {
            config,
            http_client,
        }
    }

    /// Returns a reference to the active configuration.
    pub fn config(&self) -> &PythStreamConfig {
        &self.config
    }

    /// Connects to the Pyth Hermes SSE stream and returns an active `PythEventStream`.
    pub async fn connect(&self) -> Result<PythEventStream, PythStreamError> {
        let url = self.config.build_url()?;
        let headers = self.config.build_headers()?;

        debug!(url = %url, feeds = ?self.config.feed_ids, "Connecting to Pyth Hermes SSE stream");

        let response = self
            .http_client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, url = %url, "HTTP connection failure to Pyth SSE endpoint");
                PythStreamError::Connection(e.to_string())
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %body, "Pyth Hermes SSE connection rejected by server");
            return Err(PythStreamError::Http {
                status: status.as_u16(),
                message: body,
            });
        }

        info!(
            status = %status,
            feeds_count = self.config.feed_ids.len(),
            "Successfully connected to Pyth Hermes SSE stream"
        );

        let byte_stream = response.bytes_stream();
        Ok(PythEventStream::new(byte_stream))
    }
}

/// Asynchronous stream yielding parsed `PythPriceUpdateEvent` updates or structured errors.
pub struct PythEventStream {
    byte_stream: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    parser: SseChunkParser,
    pending_items: VecDeque<Result<PythPriceUpdateEvent, PythStreamError>>,
    is_terminated: bool,
}

impl std::fmt::Debug for PythEventStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PythEventStream")
            .field("is_terminated", &self.is_terminated)
            .field("pending_items_count", &self.pending_items.len())
            .finish()
    }
}

impl PythEventStream {
    /// Creates a new `PythEventStream` wrapping an underlying HTTP byte stream.
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    {
        Self {
            byte_stream: Box::pin(stream),
            parser: SseChunkParser::new(),
            pending_items: VecDeque::new(),
            is_terminated: false,
        }
    }

    /// Asynchronously polls for the next price update event.
    ///
    /// Returns `None` when the underlying SSE connection has closed cleanly.
    pub async fn next_update(&mut self) -> Option<Result<PythPriceUpdateEvent, PythStreamError>> {
        self.next().await
    }
}

impl Stream for PythEventStream {
    type Item = Result<PythPriceUpdateEvent, PythStreamError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // First drain any events parsed from previous chunks
            if let Some(item) = self.pending_items.pop_front() {
                return Poll::Ready(Some(item));
            }

            if self.is_terminated {
                return Poll::Ready(None);
            }

            // Poll the underlying HTTP byte stream for more data
            match self.byte_stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => {
                            let parsed_raw = self.parser.feed_str(text);
                            for raw_res in parsed_raw {
                                match raw_res {
                                    Ok(raw) => {
                                        match parse_price_update_event(raw) {
                                            Ok(event) => {
                                                // Filter out empty heartbeat events from returning to caller
                                                if event.parsed.is_some() || event.binary.is_some() {
                                                    self.pending_items.push_back(Ok(event));
                                                }
                                            }
                                            Err(err) => {
                                                self.pending_items.push_back(Err(err));
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        self.pending_items.push_back(Err(err));
                                    }
                                }
                            }
                        }
                        Err(utf8_err) => {
                            error!(error = %utf8_err, "Encountered invalid UTF-8 in SSE stream chunk");
                            return Poll::Ready(Some(Err(PythStreamError::MalformedSse(
                                format!("Invalid UTF-8 chunk: {}", utf8_err),
                            ))));
                        }
                    }
                }
                Poll::Ready(Some(Err(reqwest_err))) => {
                    self.is_terminated = true;
                    error!(error = %reqwest_err, "HTTP network stream error during SSE streaming");
                    return Poll::Ready(Some(Err(PythStreamError::Connection(
                        reqwest_err.to_string(),
                    ))));
                }
                Poll::Ready(None) => {
                    self.is_terminated = true;
                    info!("Pyth Hermes SSE stream reached EOF / terminated");
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}
