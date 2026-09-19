//! Administrative API Authentication Middleware.
//!
//! Enforces that mutating/administrative endpoints require a valid administrative credential.
//! Rejects missing credentials with HTTP 401 Unauthorized.
//! Rejects invalid credentials with HTTP 401 Unauthorized.
//! Never logs or exposes credentials in traces or responses.

use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::{error::ApiError, state::AppState};

pub const X_ADMIN_KEY_HEADER: &str = "x-admin-key";

/// Middleware function to authenticate administrative requests.
pub async fn require_admin_auth(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let expected_key = state.config.admin_api_key.trim();

    // In non-production, if expected_key is empty or disabled, fail closed if empty
    if expected_key.is_empty() {
        return Err(ApiError::Unauthorized(
            "Administrative access is disabled on this cluster".to_string(),
        ));
    }

    let headers = req.headers();

    // Check 1: X-Admin-Key header
    let candidate = if let Some(key_val) = headers.get(X_ADMIN_KEY_HEADER) {
        key_val.to_str().ok()
    } else if let Some(auth_val) = headers.get(AUTHORIZATION) {
        // Check 2: Authorization: Bearer <key>
        if let Ok(auth_str) = auth_val.to_str() {
            auth_str
                .strip_prefix("Bearer ")
                .or_else(|| auth_str.strip_prefix("bearer "))
                .map(|token| token.trim())
        } else {
            None
        }
    } else {
        None
    };

    let provided = match candidate {
        Some(c) if !c.is_empty() => c,
        _ => {
            return Err(ApiError::Unauthorized(
                "Missing administrative authorization".to_string(),
            ));
        }
    };

    // Constant-time-like length & byte comparison to prevent timing attacks
    if !secure_equals(provided.as_bytes(), expected_key.as_bytes()) {
        return Err(ApiError::Unauthorized(
            "Invalid administrative authorization".to_string(),
        ));
    }

    Ok(next.run(req).await)
}

/// Constant-time byte comparison to prevent timing side-channels
fn secure_equals(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_equals() {
        assert!(secure_equals(b"secret123", b"secret123"));
        assert!(!secure_equals(b"secret123", b"secret124"));
        assert!(!secure_equals(b"secret123", b"secret"));
    }
}
