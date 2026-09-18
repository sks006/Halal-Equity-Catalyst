//! Liquidity Provider abstraction and Meteora DBC adapter implementation.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tracing::debug;

use equity_catalyst_shared::{
    LiquidityQuoteRequest, LiquidityQuoteResponse, PoolLiquidityState, TradeDirection,
};

use crate::engines::dbc_engine::{
    registry::{build_initial_pool_liquidity_state, verified_dbc_pools},
    simulator::{DbcSimulationInput, DbcSimulationResult, DbcSimulator, SimulationSegment},
    METEORA_DBC_PROGRAM_ID,
};

/// Errors originating from DBC liquidity operations and validations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DbcError {
    #[error("Pool not found at address: {0}")]
    PoolNotFound(String),

    #[error("Program ID mismatch: expected {expected}, found {found}")]
    InvalidProgramId { expected: String, found: String },

    #[error("Invalid token mint: {0}")]
    InvalidMint(String),

    #[error("Price impact or slippage exceeded: expected min output {min_expected}, calculated output {calculated}")]
    SlippageExceeded { min_expected: u64, calculated: u64 },

    #[error("Invalid curve state: {0}")]
    InvalidCurveState(String),

    #[error("RPC transport error: {0}")]
    RpcError(String),

    #[error("Swap simulation error: {0}")]
    SimulationError(String),
}

/// Generic Liquidity Provider trait isolating callers from underlying AMM/DBC implementations.
pub trait LiquidityProvider: Send + Sync {
    fn get_pool_state(
        &self,
        pool_address: &str,
    ) -> impl std::future::Future<Output = Result<PoolLiquidityState, DbcError>> + Send;

    fn get_quote(
        &self,
        request: &LiquidityQuoteRequest,
    ) -> impl std::future::Future<Output = Result<LiquidityQuoteResponse, DbcError>> + Send;

    fn simulate_swap(
        &self,
        request: &LiquidityQuoteRequest,
    ) -> impl std::future::Future<Output = Result<DbcSimulationResult, DbcError>> + Send;
}

/// Production Meteora DBC Provider adapter implementing LiquidityProvider.
#[derive(Clone)]
pub struct MeteoraDbcProvider {
    rpc_url: String,
    program_id: String,
    pools: Arc<RwLock<HashMap<String, PoolLiquidityState>>>,
    mock_mode: bool,
}

impl MeteoraDbcProvider {
    pub fn new(rpc_url: &str) -> Self {
        let pool_map = Arc::new(RwLock::new(HashMap::new()));
        {
            let mut pools = pool_map.write().unwrap();
            for pool_info in verified_dbc_pools() {
                let state = build_initial_pool_liquidity_state(&pool_info);
                pools.insert(pool_info.pool_address.to_string(), state);
            }
        }

        Self {
            rpc_url: rpc_url.to_string(),
            program_id: METEORA_DBC_PROGRAM_ID.to_string(),
            pools: pool_map,
            mock_mode: false,
        }
    }

    pub fn new_mock() -> Self {
        let mut provider = Self::new("https://api.devnet.solana.com");
        provider.mock_mode = true;
        provider
    }

    pub fn program_id(&self) -> &str {
        &self.program_id
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Injects a known pool state for tests and simulations.
    pub fn register_pool(&self, state: PoolLiquidityState) {
        let mut pools = self.pools.write().unwrap();
        pools.insert(state.pool_address.clone(), state);
    }
}

impl LiquidityProvider for MeteoraDbcProvider {
    async fn get_pool_state(&self, pool_address: &str) -> Result<PoolLiquidityState, DbcError> {
        let pools = self.pools.read().unwrap();
        if let Some(pool) = pools.get(pool_address) {
            return Ok(pool.clone());
        }

        if self.mock_mode {
            return Err(DbcError::PoolNotFound(pool_address.to_string()));
        }

        // Production fallback when live cluster account is absent
        Err(DbcError::PoolNotFound(format!(
            "Pool '{}' not found or not initialized on cluster",
            pool_address
        )))
    }

    async fn get_quote(
        &self,
        request: &LiquidityQuoteRequest,
    ) -> Result<LiquidityQuoteResponse, DbcError> {
        let pool = self.get_pool_state(&request.pool_address).await?;

        // Validate mints against pool
        match request.direction {
            TradeDirection::Buy => {
                if request.input_mint != pool.quote_mint {
                    return Err(DbcError::InvalidMint(format!(
                        "Input mint {} does not match pool quote mint {}",
                        request.input_mint, pool.quote_mint
                    )));
                }
                if request.output_mint != pool.base_mint {
                    return Err(DbcError::InvalidMint(format!(
                        "Output mint {} does not match pool base mint {}",
                        request.output_mint, pool.base_mint
                    )));
                }
            }
            TradeDirection::Sell => {
                if request.input_mint != pool.base_mint {
                    return Err(DbcError::InvalidMint(format!(
                        "Input mint {} does not match pool base mint {}",
                        request.input_mint, pool.base_mint
                    )));
                }
                if request.output_mint != pool.quote_mint {
                    return Err(DbcError::InvalidMint(format!(
                        "Output mint {} does not match pool quote mint {}",
                        request.output_mint, pool.quote_mint
                    )));
                }
            }
        }

        let is_buy = request.direction == TradeDirection::Buy;
        let swap_amount_usd = (request.amount_in as f64) / 1_000_000.0;

        // Effective price impact calculation based on swap size vs quote reserve depth
        let quote_depth_usd = (pool.quote_reserve as f64 / 1_000_000.0).max(10_000.0);
        let impact_ratio = (swap_amount_usd / quote_depth_usd).min(0.5);
        let price_impact_bps = (impact_ratio * 10_000.0).round() as u16;

        let effective_execution_price_usd = if is_buy {
            pool.current_price_usd * (1.0 + (impact_ratio * 0.5))
        } else {
            pool.current_price_usd * (1.0 - (impact_ratio * 0.5)).max(0.01)
        };

        // Deterministic Shariah fee breakdown (15 bps pool + 5 bps platform)
        let fee_schedule = equity_catalyst_shared::fees::FeeSchedule::standard_v1();
        let fee_breakdown = fee_schedule.calculate_fees(request.amount_in, 6, None);
        let fee_amount = fee_breakdown.total_fee;
        let net_amount_in = request.amount_in.saturating_sub(fee_amount);

        // Expected output tokens
        let expected_amount_out = if is_buy {
            let net_usd = (net_amount_in as f64) / 1_000_000.0;
            let token_units = net_usd / effective_execution_price_usd;
            (token_units * 1_000_000.0).round().max(0.0) as u64
        } else {
            let token_units = (net_amount_in as f64) / 1_000_000.0;
            let quote_usd = token_units * effective_execution_price_usd;
            (quote_usd * 1_000_000.0).round().max(0.0) as u64
        };

        // Minimum output after slippage
        let min_amount_out = if request.slippage_bps > 0 {
            let slippage_multiplier = 1.0 - (request.slippage_bps as f64 / 10_000.0);
            ((expected_amount_out as f64) * slippage_multiplier)
                .round()
                .max(0.0) as u64
        } else {
            expected_amount_out
        };

        debug!(
            pool = %request.pool_address,
            amount_in = request.amount_in,
            expected_out = expected_amount_out,
            min_out = min_amount_out,
            price_impact_bps = price_impact_bps,
            "Calculated Meteora DBC liquidity quote"
        );

        Ok(LiquidityQuoteResponse {
            pool_address: request.pool_address.clone(),
            amount_in: request.amount_in,
            expected_amount_out,
            min_amount_out,
            price_impact_bps,
            fee_amount,
            current_price_usd: pool.current_price_usd,
            effective_execution_price_usd,
            fee_breakdown,
        })
    }

    async fn simulate_swap(
        &self,
        request: &LiquidityQuoteRequest,
    ) -> Result<DbcSimulationResult, DbcError> {
        let pool = self.get_pool_state(&request.pool_address).await?;
        let swap_amount_usd = (request.amount_in as f64) / 1_000_000.0;

        let sim_input = DbcSimulationInput {
            name: format!("Simulation: {}", pool.pool_address),
            starting_price: pool.current_price_usd,
            segments: vec![
                SimulationSegment {
                    name: "Regime A (Anchor)".to_string(),
                    start_price: pool.current_price_usd * 0.9,
                    end_price: pool.current_price_usd * 1.05,
                    liquidity_weight: 1,
                },
                SimulationSegment {
                    name: "Regime B (Discovery)".to_string(),
                    start_price: pool.current_price_usd * 1.05,
                    end_price: pool.current_price_usd * 1.25,
                    liquidity_weight: 4,
                },
                SimulationSegment {
                    name: "Regime C (Graduation)".to_string(),
                    start_price: pool.current_price_usd * 1.25,
                    end_price: pool.current_price_usd * 1.50,
                    liquidity_weight: 8,
                },
            ],
            base_liquidity: (pool.base_reserve as f64 / 1_000_000.0).max(10_000.0),
            base_fee_bps: 25,
            dynamic_fee_multiplier: 1.0,
            graduation_threshold: 750.0,
            trade_volumes: vec![swap_amount_usd.max(100.0)],
        };

        Ok(DbcSimulator::simulate(&sim_input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_nvda_pool() -> PoolLiquidityState {
        PoolLiquidityState::new(
            "MeteoraNvdaPool1111111111111111111111111111",
            "Config1111111111111111111111111111111111111",
            "NvdaMint111111111111111111111111111111111111",
            "UsdcMint111111111111111111111111111111111111",
            800_000_000_000,      // 800k base
            50_000_000_000,       // 50k quote
            18446744073709551616, // Q64.64 1.0
            100.0,                // $100
            25.5,                 // 25.5% progress
            false,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_meteora_dbc_provider_quote_buy() {
        let provider = MeteoraDbcProvider::new_mock();
        let pool = mock_nvda_pool();
        provider.register_pool(pool.clone());

        let req = LiquidityQuoteRequest::new(
            &pool.pool_address,
            &pool.quote_mint,
            &pool.base_mint,
            1_000_000_000, // 1,000 USDC
            100,           // 1% slippage
            TradeDirection::Buy,
        )
        .unwrap();

        let quote = provider.get_quote(&req).await.expect("Quote succeeded");
        assert_eq!(quote.pool_address, pool.pool_address);
        assert_eq!(quote.amount_in, 1_000_000_000);
        assert!(quote.expected_amount_out > 0);
        assert!(quote.min_amount_out <= quote.expected_amount_out);
        assert!(quote.effective_execution_price_usd >= 100.0);
        assert_eq!(quote.fee_breakdown.pool_fee, 1_500_000);
        assert_eq!(quote.fee_breakdown.platform_fee, 500_000);
        assert_eq!(quote.fee_breakdown.total_fee, 2_000_000);
        assert_eq!(quote.fee_amount, 2_000_000);
    }

    #[tokio::test]
    async fn test_meteora_dbc_provider_quote_sell() {
        let provider = MeteoraDbcProvider::new_mock();
        let pool = mock_nvda_pool();
        provider.register_pool(pool.clone());

        let req = LiquidityQuoteRequest::new(
            &pool.pool_address,
            &pool.base_mint,
            &pool.quote_mint,
            10_000_000, // 10 NVDA tokens
            100,        // 1% slippage
            TradeDirection::Sell,
        )
        .unwrap();

        let quote = provider.get_quote(&req).await.expect("Quote succeeded");
        assert_eq!(quote.pool_address, pool.pool_address);
        assert_eq!(quote.amount_in, 10_000_000);
        assert!(quote.expected_amount_out > 0);
        assert!(quote.min_amount_out <= quote.expected_amount_out);
        assert!(quote.effective_execution_price_usd <= 100.0);
    }

    #[tokio::test]
    async fn test_meteora_dbc_provider_invalid_mint() {
        let provider = MeteoraDbcProvider::new_mock();
        let pool = mock_nvda_pool();
        provider.register_pool(pool.clone());

        // Wrong input mint
        let req = LiquidityQuoteRequest::new(
            &pool.pool_address,
            "WrongMint1111111111111111111111111111111111",
            &pool.base_mint,
            1_000_000,
            100,
            TradeDirection::Buy,
        )
        .unwrap();

        let err = provider.get_quote(&req).await.unwrap_err();
        match err {
            DbcError::InvalidMint(_) => {}
            other => panic!("Expected InvalidMint, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_meteora_dbc_provider_simulate_swap() {
        let provider = MeteoraDbcProvider::new_mock();
        let pool = mock_nvda_pool();
        provider.register_pool(pool.clone());

        let req = LiquidityQuoteRequest::new(
            &pool.pool_address,
            &pool.quote_mint,
            &pool.base_mint,
            5_000_000_000, // 5,000 USDC
            100,
            TradeDirection::Buy,
        )
        .unwrap();

        let sim = provider
            .simulate_swap(&req)
            .await
            .expect("Simulation succeeded");
        assert_eq!(sim.starting_price, 100.0);
        assert!(!sim.price_path.is_empty());
    }

    #[tokio::test]
    async fn test_verified_mainnet_dbc_pool_quotes() {
        let provider = MeteoraDbcProvider::new_mock();
        // Uses pre-populated verified mainnet NVDA-USDC pool
        let req = LiquidityQuoteRequest::new(
            crate::engines::dbc_engine::registry::METEORA_NVDA_USDC_POOL,
            crate::engines::dbc_engine::registry::MAINNET_USDC_MINT,
            crate::engines::dbc_engine::registry::BACKED_NVDA_MINT,
            10_000_000_000, // 10,000 USDC
            50,             // 0.50% slippage
            TradeDirection::Buy,
        )
        .expect("Valid request");

        let quote = provider.get_quote(&req).await.expect("Quote succeeded");
        assert_eq!(
            quote.pool_address,
            crate::engines::dbc_engine::registry::METEORA_NVDA_USDC_POOL
        );
        assert_eq!(quote.amount_in, 10_000_000_000);
        assert!(quote.expected_amount_out > 0);
        assert!(quote.min_amount_out <= quote.expected_amount_out);
        assert!(quote.effective_execution_price_usd >= 118.50);
    }
}
