//! Meteora DBC configuration and pool endpoints.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    engines::dbc_engine::{
        ComparisonSimulationRequest, DbcConfigRequest, DbcEngine, DbcSimulationInput, DbcSimulator,
    },
    error::ApiError,
    models::dbc_pool::{CreateDbcPoolRequest, DbcPoolModel},
    repositories::DbcPoolRepository,
    state::AppState,
};

/// Verified real stock asset response conforming to the hackathon checklist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedAsset {
    pub name: String,
    pub symbol: String,
    pub issuer: String,
    pub regulatory_framework: String,
    pub mint: String,
    pub decimals: u8,
    pub supported_quote_mints: Vec<String>,
    pub liquidity_venues: Vec<String>,
    pub hackathon_allowed: bool,
    pub is_invented_token: bool,
}

/// GET /dbc/assets/verified - Returns the canonical, verified tokenized equity assets.
pub async fn get_verified_assets_handler() -> Result<impl IntoResponse, ApiError> {
    let assets = vec![
        VerifiedAsset {
            name: "Backed NVIDIA".to_string(),
            symbol: "NVDAx".to_string(),
            issuer: "Backed Finance (Backed Assets GmbH, Switzerland)".to_string(),
            regulatory_framework: "Swiss DLT Act (Distributed Ledger Technology Act)".to_string(),
            mint: "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh".to_string(),
            decimals: 8,
            supported_quote_mints: vec![
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC (Mainnet)
                "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr".to_string(), // USDC (Devnet)
                "So11111111111111111111111111111111111111112".to_string(), // WSOL
            ],
            liquidity_venues: vec![
                "Jupiter Aggregator".to_string(),
                "Raydium CLMM/CPMM".to_string(),
                "Meteora DLMM".to_string(),
                "Equity Catalyst DBC".to_string(),
            ],
            hackathon_allowed: true,
            is_invented_token: false,
        },
        VerifiedAsset {
            name: "Backed Apple".to_string(),
            symbol: "AAPLx".to_string(),
            issuer: "Backed Finance (Backed Assets GmbH, Switzerland)".to_string(),
            regulatory_framework: "Swiss DLT Act".to_string(),
            mint: "XsbEhLAtcf6HdfpFZ5xEMdqW8nfAvcsP5bdudRLJzJp".to_string(),
            decimals: 8,
            supported_quote_mints: vec![
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr".to_string(),
            ],
            liquidity_venues: vec!["Jupiter".to_string(), "Raydium".to_string()],
            hackathon_allowed: true,
            is_invented_token: false,
        },
        VerifiedAsset {
            name: "Backed S&P 500".to_string(),
            symbol: "SPYx".to_string(),
            issuer: "Backed Finance (Backed Assets GmbH, Switzerland)".to_string(),
            regulatory_framework: "Swiss DLT Act".to_string(),
            mint: "XsoCS1TfEyfFhfvj8EtZ528L3CaKBDBRqRapnBbDF2W".to_string(),
            decimals: 8,
            supported_quote_mints: vec![
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr".to_string(),
            ],
            liquidity_venues: vec!["Jupiter".to_string(), "Raydium".to_string()],
            hackathon_allowed: true,
            is_invented_token: false,
        },
    ];

    Ok((StatusCode::OK, Json(assets)))
}

/// POST /dbc/configure - Validates and compiles a DBC configuration for tokenized equity launches.
pub async fn configure_dbc_handler(
    Json(request): Json<DbcConfigRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let engine = DbcEngine::new();
    let response = engine
        .configure(request)
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;

    Ok((StatusCode::OK, Json(response)))
}

/// POST /dbc/simulate - Simulates price path, slippage, and graduation for a single DBC configuration.
pub async fn simulate_dbc_handler(
    Json(request): Json<DbcSimulationInput>,
) -> Result<impl IntoResponse, ApiError> {
    let result = DbcSimulator::simulate(&request);
    Ok((StatusCode::OK, Json(result)))
}

/// POST /dbc/simulate/compare - Compares Config A vs Config B vs Default DBC across block orders.
pub async fn compare_dbc_handler(
    Json(request): Json<ComparisonSimulationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let result = DbcSimulator::compare(request);
    Ok((StatusCode::OK, Json(result)))
}

/// POST /dbc/pools - Persists a newly launched DBC pool in PostgreSQL.
pub async fn record_dbc_pool_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateDbcPoolRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = DbcPoolRepository::new(state.db_pool.clone());
    let pool = repo.create(&payload).await?;
    Ok((StatusCode::CREATED, Json(pool)))
}

/// GET /dbc/pools - Lists all launched DBC pools from PostgreSQL.
pub async fn list_dbc_pools_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<DbcPoolModel>>, ApiError> {
    let repo = DbcPoolRepository::new(state.db_pool.clone());
    let pools = repo.list_all().await?;
    Ok(Json(pools))
}

/// GET /dbc/pools/:address - Fetches a specific DBC pool by address.
pub async fn get_dbc_pool_handler(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> Result<Json<DbcPoolModel>, ApiError> {
    let repo = DbcPoolRepository::new(state.db_pool.clone());
    let pool = repo
        .find_by_pool_address(&address)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("DBC pool not found: {}", address)))?;
    Ok(Json(pool))
}
