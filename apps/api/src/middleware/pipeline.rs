//! Middleware pipeline configuration and assembly.
//!
//! Conforms to Single Responsibility Principle (SRP):
//! Responsible solely for:
//! - Applying authentication guards to administrative routes
//! - Merging route groups and 404 fallback routing
//! - Configuring and assembling the global middleware pipeline:
//!   - CORS negotiation (permissive headers/methods)
//!   - Distributed HTTP request tracing (TraceLayer)
//!   - Sliding-window rate limiting (`rate_limit_middleware`)
//!   - Default body limit (1MB payload cap against DoS)
//!   - State injection into the final Axum router

use axum::{extract::DefaultBodyLimit, middleware::from_fn_with_state, Router};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    middleware::{rate_limit_middleware, require_admin_auth},
    router::{admin_routes, fallback_handler, public_routes},
    state::AppState,
};

/// Builder and compositor for the API middleware pipeline.
pub struct MiddlewarePipeline;

impl MiddlewarePipeline {
    /// Creates the standard permissive CORS layer.
    pub fn cors_layer() -> CorsLayer {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    }

    /// Wraps administrative routes with authentication middleware.
    pub fn apply_admin_auth(
        router: Router<Arc<AppState>>,
        state: Arc<AppState>,
    ) -> Router<Arc<AppState>> {
        router.route_layer(from_fn_with_state(state, require_admin_auth))
    }

    /// Composes public and administrative routes, applies route-level auth and fallback,
    /// and applies the full outer middleware stack.
    pub fn assemble_router(state: Arc<AppState>) -> Router {
        let admin = Self::apply_admin_auth(admin_routes(), state.clone());
        let public = public_routes();

        let base_router = Router::new()
            .merge(public)
            .merge(admin)
            .fallback(fallback_handler);

        Self::apply_global_middleware(base_router, state)
    }

    /// Applies the global middleware stack and injects state.
    ///
    /// Pipeline execution order (outermost to innermost):
    /// 1. CORS header negotiation
    /// 2. TraceLayer (HTTP tracing and logging)
    /// 3. Rate limiting (sliding-window per-client limiter)
    /// 4. DefaultBodyLimit (1MB maximum payload)
    /// 5. Shared application state injection
    pub fn apply_global_middleware(router: Router<Arc<AppState>>, state: Arc<AppState>) -> Router {
        router
            .layer(DefaultBodyLimit::max(1024 * 1024))
            .layer(from_fn_with_state(state.clone(), rate_limit_middleware))
            .layer(TraceLayer::new_for_http())
            .layer(Self::cors_layer())
            .with_state(state)
    }
}

/// Builds the complete Axum router by applying the middleware pipeline to the configured routes.
pub fn build_router(state: Arc<AppState>) -> Router {
    MiddlewarePipeline::assemble_router(state)
}

/// Convenience alias to build the router with all middleware applied.
pub fn apply_middleware_pipeline(state: Arc<AppState>) -> Router {
    build_router(state)
}
