//! Bounded sliding-window rate limiting middleware for Axum/Tower.
//!
//! Enforces that clients cannot flood public or administrative endpoints.
//! Returns HTTP 429 Too Many Requests when the limit is exceeded.
//! State is bounded with automated cleanup of expired entries to prevent memory exhaustion.

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use crate::{error::ApiError, state::AppState};

#[derive(Clone, Debug)]
struct ClientHistory {
    timestamps: Vec<Instant>,
    last_seen: Instant,
}

/// Thread-safe, bounded in-memory sliding-window rate limiter.
#[derive(Clone, Debug)]
pub struct RateLimiter {
    clients: Arc<RwLock<HashMap<String, ClientHistory>>>,
    max_requests: u32,
    window: Duration,
    max_clients: usize,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window,
            max_clients: 10_000,
        }
    }

    /// Checks if a client is within rate limits. If allowed, records the request timestamp.
    pub fn check_and_record(&self, client_id: &str) -> Result<(), ApiError> {
        let now = Instant::now();
        let mut map = self
            .clients
            .write()
            .map_err(|_| ApiError::InternalServerError("Rate limiter lock poisoned".to_string()))?;

        // Periodic/threshold pruning when table grows
        if map.len() > self.max_clients {
            let cutoff = now.checked_sub(self.window).unwrap_or(now);
            map.retain(|_, history| history.last_seen > cutoff);
        }

        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        let history = map
            .entry(client_id.to_string())
            .or_insert_with(|| ClientHistory {
                timestamps: Vec::with_capacity(self.max_requests as usize + 1),
                last_seen: now,
            });

        // Prune timestamps older than window
        history.timestamps.retain(|&t| t > cutoff);
        history.last_seen = now;

        if history.timestamps.len() >= self.max_requests as usize {
            return Err(ApiError::TooManyRequests(format!(
                "Rate limit exceeded: maximum {} requests per {:?}",
                self.max_requests, self.window
            )));
        }

        history.timestamps.push(now);
        Ok(())
    }

    /// Resets state (useful for tests)
    pub fn reset(&self) {
        if let Ok(mut map) = self.clients.write() {
            map.clear();
        }
    }
}

/// Extracts a client identifier from headers (X-Forwarded-For, X-Real-IP, or default).
pub fn extract_client_id(headers: &HeaderMap) -> String {
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            if let Some(first_ip) = s.split(',').next() {
                let trimmed = first_ip.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    "default-client".to_string()
}

/// Global rate limiting middleware function.
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let client_id = extract_client_id(req.headers());
    state.rate_limiter().check_and_record(&client_id)?;

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit_and_blocks_burst() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));
        let client = "192.168.1.100";

        assert!(limiter.check_and_record(client).is_ok());
        assert!(limiter.check_and_record(client).is_ok());
        assert!(limiter.check_and_record(client).is_ok());

        // 4th request must be rejected with TooManyRequests
        let err = limiter.check_and_record(client).unwrap_err();
        match err {
            ApiError::TooManyRequests(msg) => {
                assert!(msg.contains("Rate limit exceeded"));
            }
            other => panic!("Expected TooManyRequests, got {:?}", other),
        }

        // Another client should still be allowed
        assert!(limiter.check_and_record("10.0.0.1").is_ok());
    }

    #[test]
    fn test_rate_limiter_recovers_after_window() {
        let limiter = RateLimiter::new(2, Duration::from_millis(50));
        let client = "192.168.1.200";

        assert!(limiter.check_and_record(client).is_ok());
        assert!(limiter.check_and_record(client).is_ok());
        assert!(limiter.check_and_record(client).is_err());

        // Sleep past window
        std::thread::sleep(Duration::from_millis(60));

        // Should recover
        assert!(limiter.check_and_record(client).is_ok());
    }
}
