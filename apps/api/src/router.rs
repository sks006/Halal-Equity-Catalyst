//! Router configuration and middleware pipeline.

use axum::{
    http::Uri,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    error::ApiError,
    routes::{
        configure_dbc_handler, create_event_handler, create_or_update_policy_handler, create_vault_handler,
        detailed_health_handler, evaluate_quote_handler, get_policy_handler, get_price_handler,
        get_vault_handler, health_handler, list_events_handler, list_executions_handler,
        list_pending_events_handler, list_policies_handler, list_vault_events_handler,
        list_vault_executions_handler, list_vaults_handler, ready_handler,
    },
    state::AppState,
};

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_handler))
        .route("/health/detailed", get(detailed_health_handler))
        .route("/health/monitor", get(detailed_health_handler))
        .route("/ready", get(ready_handler))
        .route("/oracle/price/:symbol", get(get_price_handler))
        .route("/quotes/evaluate", post(evaluate_quote_handler))
        .route("/dbc/configure", post(configure_dbc_handler))
        .route("/vaults", get(list_vaults_handler).post(create_vault_handler))
        .route("/vaults/:address", get(get_vault_handler))
        .route("/vaults/:address/policy", get(get_policy_handler).put(create_or_update_policy_handler))
        .route("/policies", get(list_policies_handler).post(create_or_update_policy_handler))
        .route("/policies/:vault_address", get(get_policy_handler))
        .route("/events", get(list_events_handler).post(create_event_handler))
        .route("/events/pending", get(list_pending_events_handler))
        .route("/vaults/:address/events", get(list_vault_events_handler))
        .route("/executions", get(list_executions_handler))
        .route("/vaults/:address/executions", get(list_vault_executions_handler))
        .fallback(fallback_handler)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn fallback_handler(uri: Uri) -> ApiError {
    ApiError::NotFound(format!("Route not found: {}", uri.path()))
}
