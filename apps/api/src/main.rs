//! Main executable entrypoint for Equity Catalyst API.

use equity_catalyst_api::{
    config::Config, create_db_pool, router::create_router, state::AppState,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "equity_catalyst_api=debug,tower_http=info".into()),
        )
        .init();

    // Load configuration
    let config = Config::from_env();
    let addr = config.address();

    info!(
        cluster = %config.solana_cluster,
        address = %addr,
        "Initializing Equity Catalyst API server"
    );

    // Initialize PostgreSQL connection pool
    let db_pool = match create_db_pool(&config.database_url) {
        Ok(pool) => {
            info!("PostgreSQL connection pool initialized successfully");
            pool
        }
        Err(err) => {
            error!(error = %err, "Failed to initialize PostgreSQL pool");
            return Err(err);
        }
    };

    // Initialize Redis client
    let redis_client = match redis::Client::open(config.redis_url.as_str()) {
        Ok(client) => {
            info!(redis_url = %config.redis_url, "Redis client configured");
            Some(client)
        }
        Err(err) => {
            warn!(error = %err, "Redis client could not be configured");
            None
        }
    };

    // Initialize state & router
    let state = Arc::new(AppState::new(config, db_pool, redis_client));
    let app = create_router(state);

    // Bind TCP listener
    let listener = TcpListener::bind(&addr).await?;
    info!("Equity Catalyst API listening on http://{}", addr);

    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Equity Catalyst API shutdown completed cleanly");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C interrupt signal, shutting down...");
        },
        _ = terminate => {
            info!("Received SIGTERM terminate signal, shutting down...");
        },
    }
}
