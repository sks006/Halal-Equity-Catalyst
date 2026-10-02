//! Router configuration.
//!
//! Conforms to Single Responsibility Principle (SRP):
//! Responsible solely for declaring endpoint paths, HTTP method bindings,
//! and 404 fallback routing. Contains NO middleware or transport layers.
//!
//! Middleware pipeline application is decoupled and managed by `crate::pipeline`.

use axum::{
    http::Uri,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::{
    error::ApiError,
    routes::{
        compare_dbc_handler, configure_dbc_handler, create_event_handler,
        create_or_update_policy_handler, create_vault_handler, detailed_health_handler,
        evaluate_quote_handler, get_dbc_pool_handler, get_market_data_handler, get_policy_handler,
        get_price_handler, get_vault_handler, get_verified_assets_handler, health_handler,
        list_dbc_pools_handler, list_events_handler, list_executions_handler,
        list_pending_events_handler, list_policies_handler, list_vault_events_handler,
        list_vault_executions_handler, list_vaults_handler, market_data_ws_handler,
        metrics_handler, ready_handler, record_dbc_pool_handler, simulate_dbc_handler,
    },
    state::AppState,
};

/// Configures administrative and state-mutating endpoints (pure route definitions).
pub fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/dbc/configure", post(configure_dbc_handler))
        .route("/dbc/pools", post(record_dbc_pool_handler))
        .route("/vaults", post(create_vault_handler))
        .route(
            "/vaults/:address/policy",
            post(create_or_update_policy_handler).put(create_or_update_policy_handler),
        )
        .route("/policies", post(create_or_update_policy_handler))
        .route("/events", post(create_event_handler))
}

/// Configures public read-only and simulation endpoints (pure route definitions).
pub fn public_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_handler))
        .route("/health/detailed", get(detailed_health_handler))
        .route("/health/monitor", get(detailed_health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        .route("/oracle/price/:symbol", get(get_price_handler))
        .route("/market-data/ws", get(market_data_ws_handler))
        .route("/market-data/:asset_id", get(get_market_data_handler))
        .route("/quotes/evaluate", post(evaluate_quote_handler))
        .route("/dbc/simulate", post(simulate_dbc_handler))
        .route("/dbc/simulate/compare", post(compare_dbc_handler))
        .route("/dbc/assets/verified", get(get_verified_assets_handler))
        .route("/dbc/pools", get(list_dbc_pools_handler))
        .route("/dbc/pools/:address", get(get_dbc_pool_handler))
        .route("/vaults", get(list_vaults_handler))
        .route("/vaults/:address", get(get_vault_handler))
        .route("/vaults/:address/policy", get(get_policy_handler))
        .route("/policies", get(list_policies_handler))
        .route("/policies/:vault_address", get(get_policy_handler))
        .route("/events", get(list_events_handler))
        .route("/events/pending", get(list_pending_events_handler))
        .route("/vaults/:address/events", get(list_vault_events_handler))
        .route("/executions", get(list_executions_handler))
        .route(
            "/vaults/:address/executions",
            get(list_vault_executions_handler),
        )
}

/// Fallback 404 handler for undefined endpoints.
pub async fn fallback_handler(uri: Uri) -> ApiError {
    ApiError::NotFound(format!("Route not found: {}", uri.path()))
}

/// Re-export create_router for backwards compatibility by delegating to the middleware pipeline.
pub fn create_router(state: Arc<AppState>) -> Router {
    crate::pipeline::build_router(state)
}
