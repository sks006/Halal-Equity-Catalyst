//! Custom middleware modules for authentication, security, and rate limiting.

pub mod auth;
pub mod rate_limit;

pub use auth::{require_admin_auth, X_ADMIN_KEY_HEADER};
pub use rate_limit::{rate_limit_middleware, RateLimiter};
