use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use thiserror::Error;
use tracing::debug;

use equity_catalyst_shared::{
    LiquidityQuoteRequest, LiquidityQuoteResponse, PoolLiquidityState, TradeDirection,
};
use equity_catalyst_solana::SolanaRpcClient;

use crate::engines::dbc_engine::{
    pricing::price_from_sqrt_price_q64,
    registry::{
        build_initial_pool_liquidity_state, find_verified_pool_by_address, verified_dbc_pools,
    },
    simulator::{DbcSimulationInput, DbcSimulationResult, DbcSimulator, SimulationSegment},
    METEORA_DBC_PROGRAM_ID,
};

/// Discriminator for Meteora DBC VirtualPool account: Sha256("account:VirtualPool")[..8]
pub const VIRTUAL_POOL_DISCRIMINATOR: [u8; 8] = [213, 224, 5, 209, 98, 69, 119, 92];

/// Discriminator for Meteora DBC PoolConfig account: Sha256("account:PoolConfig")[..8]
pub const POOL_CONFIG_DISCRIMINATOR: [u8; 8] = [26, 108, 14, 123, 116, 230, 129, 43];

/// Minimum byte size of on-chain Meteora DBC VirtualPool account data
pub const METEORA_DBC_POOL_MIN_SIZE: usize = 309;

/// Minimum byte size of on-chain Meteora DBC PoolConfig account data
pub const METEORA_DBC_CONFIG_MIN_SIZE: usize = 40;

/// Errors originating from DBC liquidity operations, on-chain RPC fetching, and registry validations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DbcError {
    #[error("Pool not permitted or not found in canonical registry: {0}")]
    PoolNotPermitted(String),

    #[error("Pool account not found on-chain: {0}")]
    PoolNotFound(String),

    #[error("Program ID mismatch for pool {pool}: expected {expected}, found {found}")]
    InvalidProgramId {
        pool: String,
        expected: String,
        found: String,
    },

    #[error("Base mint mismatch for pool {pool}: expected {expected}, found {found}")]
    BaseMintMismatch {
        pool: String,
        expected: String,
        found: String,
    },

    #[error("Quote mint mismatch for pool {pool}: expected {expected}, found {found}")]
    QuoteMintMismatch {
        pool: String,
        expected: String,
        found: String,
    },

    #[error("Config account mismatch for pool {pool}: expected {expected}, found {found}")]
    ConfigMismatch {
        pool: String,
        expected: String,
        found: String,
    },

    #[error("RPC transport error for pool {pool}: {message}")]
    RpcError { pool: String, message: String },

    #[error("RPC call timed out for pool {pool} after {timeout_ms}ms")]
    RpcTimeout { pool: String, timeout_ms: u64 },

    #[error("Account decoding failed for pool {pool}: {message}")]
    AccountDecodeError { pool: String, message: String },

    #[error("Invalid token mint: {0}")]
    InvalidMint(String),

    #[error("Price impact or slippage exceeded: expected min output {min_expected}, calculated output {calculated}")]
    SlippageExceeded { min_expected: u64, calculated: u64 },

    #[error("Invalid curve state: {0}")]
    InvalidCurveState(String),

    #[error("Swap simulation error: {0}")]
    SimulationError(String),
}

/// Decoded on-chain VirtualPool state from Meteora Dynamic Bonding Curve program.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedDbcPool {
    pub config: Pubkey,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub base_reserve: u64,
    pub quote_reserve: u64,
    pub sqrt_price: u128,
    pub activation_point: u64,
    pub pool_type: u8,
    pub is_migrated: bool,
    pub migration_progress: u8,
}

/// Decodes on-chain binary account data into a `DecodedDbcPool` representation.
pub fn decode_on_chain_dbc_pool(
    pool_address: &str,
    data: &[u8],
) -> Result<DecodedDbcPool, DbcError> {
    if data.len() < METEORA_DBC_POOL_MIN_SIZE {
        return Err(DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: format!(
                "Account data too short: expected at least {} bytes, got {}",
                METEORA_DBC_POOL_MIN_SIZE,
                data.len()
            ),
        });
    }

    if data[0..8] != VIRTUAL_POOL_DISCRIMINATOR {
        return Err(DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: format!(
                "Invalid pool discriminator: expected {:?}, found {:?}",
                VIRTUAL_POOL_DISCRIMINATOR,
                &data[0..8]
            ),
        });
    }

    let config = Pubkey::new_from_array(data[72..104].try_into().map_err(|e| {
        DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: format!("Failed to read config pubkey: {}", e),
        }
    })?);

    let creator = Pubkey::new_from_array(data[104..136].try_into().map_err(|e| {
        DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: format!("Failed to read creator pubkey: {}", e),
        }
    })?);

    let base_mint = Pubkey::new_from_array(data[136..168].try_into().map_err(|e| {
        DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: format!("Failed to read base_mint pubkey: {}", e),
        }
    })?);

    let base_vault = Pubkey::new_from_array(data[168..200].try_into().map_err(|e| {
        DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: format!("Failed to read base_vault pubkey: {}", e),
        }
    })?);

    let quote_vault = Pubkey::new_from_array(data[200..232].try_into().map_err(|e| {
        DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: format!("Failed to read quote_vault pubkey: {}", e),
        }
    })?);

    let base_reserve = u64::from_le_bytes(data[232..240].try_into().map_err(|_| {
        DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: "Failed to read base_reserve u64".into(),
        }
    })?);

    let quote_reserve = u64::from_le_bytes(data[240..248].try_into().map_err(|_| {
        DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: "Failed to read quote_reserve u64".into(),
        }
    })?);

    let sqrt_price = u128::from_le_bytes(data[280..296].try_into().map_err(|_| {
        DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: "Failed to read sqrt_price u128".into(),
        }
    })?);

    let activation_point = u64::from_le_bytes(data[296..304].try_into().map_err(|_| {
        DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: "Failed to read activation_point u64".into(),
        }
    })?);

    let pool_type = data[304];
    let is_migrated = data[305] != 0;
    let migration_progress = data[308];

    Ok(DecodedDbcPool {
        config,
        creator,
        base_mint,
        base_vault,
        quote_vault,
        base_reserve,
        quote_reserve,
        sqrt_price,
        activation_point,
        pool_type,
        is_migrated,
        migration_progress,
    })
}

/// Encodes a `DecodedDbcPool` into 424 bytes of binary account data matching Meteora on-chain layout.
pub fn encode_on_chain_dbc_pool(pool: &DecodedDbcPool) -> Vec<u8> {
    let mut data = vec![0u8; 424];
    data[0..8].copy_from_slice(&VIRTUAL_POOL_DISCRIMINATOR);
    data[72..104].copy_from_slice(pool.config.as_ref());
    data[104..136].copy_from_slice(pool.creator.as_ref());
    data[136..168].copy_from_slice(pool.base_mint.as_ref());
    data[168..200].copy_from_slice(pool.base_vault.as_ref());
    data[200..232].copy_from_slice(pool.quote_vault.as_ref());
    data[232..240].copy_from_slice(&pool.base_reserve.to_le_bytes());
    data[240..248].copy_from_slice(&pool.quote_reserve.to_le_bytes());
    data[280..296].copy_from_slice(&pool.sqrt_price.to_le_bytes());
    data[296..304].copy_from_slice(&pool.activation_point.to_le_bytes());
    data[304] = pool.pool_type;
    data[305] = if pool.is_migrated { 1 } else { 0 };
    data[308] = pool.migration_progress;
    data
}

/// Decodes on-chain binary account data of a `PoolConfig` account to extract the canonical quote mint.
pub fn decode_on_chain_pool_config(config_address: &str, data: &[u8]) -> Result<Pubkey, DbcError> {
    if data.len() < METEORA_DBC_CONFIG_MIN_SIZE {
        return Err(DbcError::AccountDecodeError {
            pool: config_address.to_string(),
            message: format!(
                "Config account data too short: expected at least {} bytes, got {}",
                METEORA_DBC_CONFIG_MIN_SIZE,
                data.len()
            ),
        });
    }

    if data[0..8] != POOL_CONFIG_DISCRIMINATOR {
        return Err(DbcError::AccountDecodeError {
            pool: config_address.to_string(),
            message: format!(
                "Invalid config discriminator: expected {:?}, found {:?}",
                POOL_CONFIG_DISCRIMINATOR,
                &data[0..8]
            ),
        });
    }

    let quote_mint = Pubkey::new_from_array(data[8..40].try_into().map_err(|e| {
        DbcError::AccountDecodeError {
            pool: config_address.to_string(),
            message: format!("Failed to read quote_mint pubkey from config: {}", e),
        }
    })?);

    Ok(quote_mint)
}

/// Encodes a `PoolConfig` account with the specified quote mint into binary data.
pub fn encode_on_chain_pool_config(quote_mint: &Pubkey) -> Vec<u8> {
    let mut data = vec![0u8; 1048];
    data[0..8].copy_from_slice(&POOL_CONFIG_DISCRIMINATOR);
    data[8..40].copy_from_slice(quote_mint.as_ref());
    data
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
///
/// In production mode (`mock_mode: false`):
/// - Live pool state is fetched directly from Solana RPC.
/// - Hardcoded or seeded memory reserves are NEVER used.
/// - Pool address, program ID, base mint, and quote mint are strictly verified against the approved registry.
/// - RPC failure or timeout results in immediate error (NO QUOTE).
#[derive(Clone)]
pub struct MeteoraDbcProvider {
    rpc_client: Arc<SolanaRpcClient>,
    rpc_url: String,
    program_id: String,
    timeout: Duration,
    mock_mode: bool,
    test_pools: Arc<RwLock<HashMap<String, PoolLiquidityState>>>,
}

impl MeteoraDbcProvider {
    /// Constructs a production Meteora DBC provider connected to configured Solana RPC.
    /// Does NOT seed initial pool states from memory.
    pub fn new(rpc_url: &str) -> Self {
        Self::new_with_timeout(rpc_url, Duration::from_secs(5))
    }

    /// Constructs a production Meteora DBC provider with an explicit RPC timeout.
    pub fn new_with_timeout(rpc_url: &str, timeout: Duration) -> Self {
        let rpc_client = Arc::new(SolanaRpcClient::new(rpc_url).with_timeout(timeout));
        Self {
            rpc_client,
            rpc_url: rpc_url.to_string(),
            program_id: METEORA_DBC_PROGRAM_ID.to_string(),
            timeout,
            mock_mode: false,
            test_pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Constructs a production provider wrapping a shared `SolanaRpcClient`.
    pub fn new_with_rpc_client(rpc_client: Arc<SolanaRpcClient>, timeout: Duration) -> Self {
        let rpc_url = rpc_client.rpc_url().to_string();
        Self {
            rpc_client,
            rpc_url,
            program_id: METEORA_DBC_PROGRAM_ID.to_string(),
            timeout,
            mock_mode: false,
            test_pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Isolated mock provider for unit tests without live RPC network.
    pub fn new_mock() -> Self {
        let rpc_client = Arc::new(SolanaRpcClient::new("http://127.0.0.1:8899"));
        Self {
            rpc_client,
            rpc_url: "http://127.0.0.1:8899".to_string(),
            program_id: METEORA_DBC_PROGRAM_ID.to_string(),
            timeout: Duration::from_secs(5),
            mock_mode: true,
            test_pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Isolated mock provider pre-seeded with initial verified pool liquidity states strictly for offline unit tests.
    pub fn new_mock_seeded() -> Self {
        let provider = Self::new_mock();
        {
            let mut pools = provider.test_pools.write().unwrap();
            for pool_info in verified_dbc_pools() {
                let state = build_initial_pool_liquidity_state(&pool_info);
                pools.insert(pool_info.pool_address.to_string(), state);
            }
        }
        provider
    }

    pub fn program_id(&self) -> &str {
        &self.program_id
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn is_mock_mode(&self) -> bool {
        self.mock_mode
    }

    /// Injects a known pool state strictly for offline tests and simulations.
    pub fn register_pool(&self, state: PoolLiquidityState) {
        let mut pools = self.test_pools.write().unwrap();
        pools.insert(state.pool_address.clone(), state);
    }
}

impl LiquidityProvider for MeteoraDbcProvider {
    async fn get_pool_state(&self, pool_address: &str) -> Result<PoolLiquidityState, DbcError> {
        // 1. In mock mode (isolated tests), return from in-memory test fixtures if registered
        if self.mock_mode {
            let pools = self.test_pools.read().unwrap();
            if let Some(pool) = pools.get(pool_address) {
                return Ok(pool.clone());
            }
            return Err(DbcError::PoolNotFound(pool_address.to_string()));
        }

        // 2. In production mode: Verify pool address against approved canonical registry
        let registry_pool = find_verified_pool_by_address(pool_address).ok_or_else(|| {
            DbcError::PoolNotPermitted(format!(
                "Pool '{}' is not registered in the canonical verified DBC pool registry",
                pool_address
            ))
        })?;

        // 3. Connect to configured Solana RPC and fetch live pool account
        let pool_pubkey = Pubkey::from_str(pool_address).map_err(|e| {
            DbcError::PoolNotFound(format!(
                "Invalid pool public key format '{}': {}",
                pool_address, e
            ))
        })?;

        let pool_account_res =
            tokio::time::timeout(self.timeout, self.rpc_client.get_account_info(&pool_pubkey))
                .await
                .map_err(|_| DbcError::RpcTimeout {
                    pool: pool_address.to_string(),
                    timeout_ms: self.timeout.as_millis() as u64,
                })?
                .map_err(|e| DbcError::RpcError {
                    pool: pool_address.to_string(),
                    message: e.to_string(),
                })?;

        let pool_account = pool_account_res.ok_or_else(|| {
            DbcError::PoolNotFound(format!(
                "Pool account '{}' not found on cluster",
                pool_address
            ))
        })?;

        // 4. Verify program ID (pool account owner)
        let owner_str = pool_account.owner.to_string();
        if owner_str != self.program_id {
            return Err(DbcError::InvalidProgramId {
                pool: pool_address.to_string(),
                expected: self.program_id.clone(),
                found: owner_str,
            });
        }

        // 5. Decode on-chain VirtualPool state
        let decoded_pool = decode_on_chain_dbc_pool(pool_address, &pool_account.data)?;

        // 6. Verify base mint against registry
        let onchain_base_mint = decoded_pool.base_mint.to_string();
        if onchain_base_mint != registry_pool.base_mint {
            return Err(DbcError::BaseMintMismatch {
                pool: pool_address.to_string(),
                expected: registry_pool.base_mint.to_string(),
                found: onchain_base_mint,
            });
        }

        // 7. Verify config account address against registry
        let onchain_config = decoded_pool.config.to_string();
        if onchain_config != registry_pool.config_address {
            return Err(DbcError::ConfigMismatch {
                pool: pool_address.to_string(),
                expected: registry_pool.config_address.to_string(),
                found: onchain_config,
            });
        }

        // 8. Fetch on-chain Config account to verify quote mint
        let config_pubkey = decoded_pool.config;
        let config_account_res = tokio::time::timeout(
            self.timeout,
            self.rpc_client.get_account_info(&config_pubkey),
        )
        .await
        .map_err(|_| DbcError::RpcTimeout {
            pool: pool_address.to_string(),
            timeout_ms: self.timeout.as_millis() as u64,
        })?
        .map_err(|e| DbcError::RpcError {
            pool: pool_address.to_string(),
            message: format!("Failed to fetch config account: {}", e),
        })?;

        let config_account = config_account_res.ok_or_else(|| DbcError::AccountDecodeError {
            pool: pool_address.to_string(),
            message: format!("Config account '{}' not found on-chain", onchain_config),
        })?;

        let config_owner_str = config_account.owner.to_string();
        if config_owner_str != self.program_id {
            return Err(DbcError::InvalidProgramId {
                pool: pool_address.to_string(),
                expected: self.program_id.clone(),
                found: config_owner_str,
            });
        }

        let onchain_quote_mint =
            decode_on_chain_pool_config(&onchain_config, &config_account.data)?.to_string();
        if onchain_quote_mint != registry_pool.quote_mint {
            return Err(DbcError::QuoteMintMismatch {
                pool: pool_address.to_string(),
                expected: registry_pool.quote_mint.to_string(),
                found: onchain_quote_mint,
            });
        }

        // 9. Derive live market values strictly from on-chain Solana state
        let current_price_usd = if decoded_pool.sqrt_price > 0 {
            price_from_sqrt_price_q64(
                decoded_pool.sqrt_price,
                registry_pool.base_decimals,
                registry_pool.quote_decimals,
            )
        } else {
            registry_pool.initial_price_usd
        };

        if current_price_usd <= 0.0 {
            return Err(DbcError::InvalidCurveState(format!(
                "Decoded on-chain pool price must be positive, got {}",
                current_price_usd
            )));
        }

        let curve_progress_pct = decoded_pool.migration_progress as f64;

        PoolLiquidityState::new(
            pool_address,
            onchain_config,
            onchain_base_mint,
            onchain_quote_mint,
            decoded_pool.base_reserve,
            decoded_pool.quote_reserve,
            decoded_pool.sqrt_price,
            current_price_usd,
            curve_progress_pct,
            decoded_pool.is_migrated,
        )
        .map_err(|e| DbcError::InvalidCurveState(e.to_string()))
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

        // Deterministic Shariah fee breakdown (15 bps pool + 5 bps platform)
        // Calculated strictly using pure integer arithmetic
        let fee_schedule = equity_catalyst_shared::fees::FeeSchedule::standard_v1();
        let fee_breakdown = fee_schedule.calculate_fees(request.amount_in, 6, None);
        let fee_amount = fee_breakdown.total_fee;
        let net_amount_in = request.amount_in.saturating_sub(fee_amount);

        if net_amount_in == 0 {
            return Err(DbcError::InvalidCurveState(
                "Net swap input amount after fees is zero".into(),
            ));
        }

        // Output reserves based on swap direction
        let (input_reserve, output_reserve) = if is_buy {
            (pool.quote_reserve, pool.base_reserve)
        } else {
            (pool.base_reserve, pool.quote_reserve)
        };

        if input_reserve == 0 || output_reserve == 0 {
            return Err(DbcError::InvalidCurveState(format!(
                "Pool {} reserves must be non-zero (base: {}, quote: {})",
                pool.pool_address, pool.base_reserve, pool.quote_reserve
            )));
        }

        // Expected output tokens computed strictly using pure integer math (Task 13)
        // Constant-product AMM: expected_amount_out = (output_reserve * net_amount_in) / (input_reserve + net_amount_in)
        let numerator = (output_reserve as u128)
            .checked_mul(net_amount_in as u128)
            .ok_or_else(|| {
                DbcError::SimulationError("Overflow in output calculation numerator".into())
            })?;
        let denominator = (input_reserve as u128)
            .checked_add(net_amount_in as u128)
            .ok_or_else(|| {
                DbcError::SimulationError("Overflow in output calculation denominator".into())
            })?;

        let expected_amount_out = (numerator / denominator) as u64;

        if expected_amount_out == 0 {
            return Err(DbcError::InvalidCurveState(
                "Calculated swap output amount is zero".into(),
            ));
        }

        // Minimum output after slippage computed strictly using pure integer math (Task 13)
        let min_amount_out = if request.slippage_bps > 0 {
            if request.slippage_bps > 10_000 {
                return Err(DbcError::SlippageExceeded {
                    min_expected: 0,
                    calculated: expected_amount_out,
                });
            }
            let factor = 10_000u128.saturating_sub(request.slippage_bps as u128);
            let min_out = ((expected_amount_out as u128) * factor) / 10_000u128;
            min_out as u64
        } else {
            expected_amount_out
        };

        // Price impact bps computed using pure integer ratio
        let impact_bps_calc = (((net_amount_in as u128) * 10_000u128) / denominator) as u16;
        let price_impact_bps = impact_bps_calc.min(5_000);

        // Informational price display fields
        let impact_ratio = price_impact_bps as f64 / 10_000.0;
        let effective_execution_price_usd = if is_buy {
            pool.current_price_usd * (1.0 + (impact_ratio * 0.5))
        } else {
            (pool.current_price_usd * (1.0 - (impact_ratio * 0.5))).max(0.01)
        };

        debug!(
            pool = %request.pool_address,
            amount_in = request.amount_in,
            expected_out = expected_amount_out,
            min_out = min_amount_out,
            price_impact_bps = price_impact_bps,
            "Calculated Meteora DBC liquidity quote with live on-chain state"
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
        let provider = MeteoraDbcProvider::new_mock_seeded();
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

    #[tokio::test]
    async fn test_production_provider_fails_on_rpc_without_fallback_no_quote() {
        // Production provider pointing to non-existent RPC endpoint with 100ms timeout
        let provider = MeteoraDbcProvider::new_with_timeout(
            "http://127.0.0.1:19999",
            Duration::from_millis(100),
        );
        assert!(!provider.is_mock_mode());

        let req = LiquidityQuoteRequest::new(
            crate::engines::dbc_engine::registry::METEORA_NVDA_USDC_POOL,
            crate::engines::dbc_engine::registry::MAINNET_USDC_MINT,
            crate::engines::dbc_engine::registry::BACKED_NVDA_MINT,
            1_000_000_000,
            50,
            TradeDirection::Buy,
        )
        .expect("Valid request");

        // Production quote MUST fail when RPC fails. Zero fallback to seeded in-memory state.
        let quote_err = provider.get_quote(&req).await.unwrap_err();
        match quote_err {
            DbcError::RpcError { pool, .. } | DbcError::RpcTimeout { pool, .. } => {
                assert_eq!(
                    pool,
                    crate::engines::dbc_engine::registry::METEORA_NVDA_USDC_POOL
                );
            }
            other => panic!("Expected RpcError or RpcTimeout, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_production_unregistered_pool_rejected() {
        let provider = MeteoraDbcProvider::new("https://api.mainnet-beta.solana.com");
        let unpermitted_pool = "11111111111111111111111111111111";

        let err = provider.get_pool_state(unpermitted_pool).await.unwrap_err();
        match err {
            DbcError::PoolNotPermitted(addr) => {
                assert!(addr.contains(unpermitted_pool));
            }
            other => panic!("Expected PoolNotPermitted, got {:?}", other),
        }
    }

    #[test]
    fn test_on_chain_dbc_pool_encoding_and_decoding() {
        let pool_addr = "JCqWLp5RAaC3FPFX8MoAt7yRuPqRW2W9ZG3Byigxd1A7";
        let config_pk = Pubkey::new_unique();
        let creator_pk = Pubkey::new_unique();
        let base_mint_pk = Pubkey::new_unique();
        let base_vault_pk = Pubkey::new_unique();
        let quote_vault_pk = Pubkey::new_unique();

        let original = DecodedDbcPool {
            config: config_pk,
            creator: creator_pk,
            base_mint: base_mint_pk,
            base_vault: base_vault_pk,
            quote_vault: quote_vault_pk,
            base_reserve: 80_000_000_000_000,
            quote_reserve: 150_000_000_000,
            sqrt_price: 18446744073709551616,
            activation_point: 100,
            pool_type: 0,
            is_migrated: false,
            migration_progress: 18,
        };

        let encoded = encode_on_chain_dbc_pool(&original);
        assert_eq!(encoded.len(), 424);

        let decoded = decode_on_chain_dbc_pool(pool_addr, &encoded).expect("Decode must succeed");
        assert_eq!(decoded, original);

        // Test truncated payload rejection
        let truncated = &encoded[..200];
        let err_truncated = decode_on_chain_dbc_pool(pool_addr, truncated).unwrap_err();
        match err_truncated {
            DbcError::AccountDecodeError { message, .. } => {
                assert!(message.contains("too short"));
            }
            other => panic!("Expected AccountDecodeError, got {:?}", other),
        }

        // Test invalid discriminator rejection
        let mut corrupted_disc = encoded.clone();
        corrupted_disc[0] ^= 0xFF;
        let err_disc = decode_on_chain_dbc_pool(pool_addr, &corrupted_disc).unwrap_err();
        match err_disc {
            DbcError::AccountDecodeError { message, .. } => {
                assert!(message.contains("Invalid pool discriminator"));
            }
            other => panic!("Expected AccountDecodeError, got {:?}", other),
        }
    }

    #[test]
    fn test_on_chain_pool_config_encoding_and_decoding() {
        let config_addr = "ASv4E2yuTiE5rsWnUG5LWvVYVQ8UnakiYTwgHtz5pf6b";
        let quote_mint = Pubkey::new_unique();

        let encoded = encode_on_chain_pool_config(&quote_mint);
        assert_eq!(encoded.len(), 1048);

        let decoded_mint =
            decode_on_chain_pool_config(config_addr, &encoded).expect("Decode must succeed");
        assert_eq!(decoded_mint, quote_mint);

        // Test truncated config
        let truncated = &encoded[..30];
        let err_truncated = decode_on_chain_pool_config(config_addr, truncated).unwrap_err();
        match err_truncated {
            DbcError::AccountDecodeError { message, .. } => {
                assert!(message.contains("too short"));
            }
            other => panic!("Expected AccountDecodeError, got {:?}", other),
        }

        // Test corrupted config discriminator
        let mut corrupted = encoded.clone();
        corrupted[0] ^= 0xFF;
        let err_disc = decode_on_chain_pool_config(config_addr, &corrupted).unwrap_err();
        match err_disc {
            DbcError::AccountDecodeError { message, .. } => {
                assert!(message.contains("Invalid config discriminator"));
            }
            other => panic!("Expected AccountDecodeError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_quote_pure_integer_math_exactness() {
        let provider = MeteoraDbcProvider::new_mock();
        let pool = mock_nvda_pool();
        provider.register_pool(pool.clone());

        // Base reserve: 800_000_000_000
        // Quote reserve: 50_000_000_000
        // Amount in: 1_000_000_000 (USDC, 6 decimals)
        // Fee schedule standard v1: 20 bps (15 bps pool + 5 bps platform) = 2_000_000 fee
        // Net amount in = 998_000_000
        // Expected out = (800_000_000_000 * 998_000_000) / (50_000_000_000 + 998_000_000)
        // Expected out = 798_400_000_000_000_000_000 / 50_998_000_000 = 15_655_515_902
        // Min out at 100 bps (1%) slippage: 15_655_515_902 * 9_900 / 10_000 = 15_498_960_742
        let req = LiquidityQuoteRequest::new(
            &pool.pool_address,
            &pool.quote_mint,
            &pool.base_mint,
            1_000_000_000,
            100, // 1%
            TradeDirection::Buy,
        )
        .unwrap();

        let quote = provider.get_quote(&req).await.expect("Quote succeeded");

        assert_eq!(quote.amount_in, 1_000_000_000);
        assert_eq!(quote.fee_amount, 2_000_000);
        assert_eq!(quote.expected_amount_out, 15_655_515_902);
        assert_eq!(quote.min_amount_out, 15_498_960_742);
        assert!(quote.min_amount_out <= quote.expected_amount_out);
    }

    #[tokio::test]
    async fn test_quote_zero_reserves_rejected() {
        let provider = MeteoraDbcProvider::new_mock();
        let empty_pool = PoolLiquidityState::new(
            "EmptyPool111111111111111111111111111111111",
            "Config1111111111111111111111111111111111111",
            "BaseMint1111111111111111111111111111111111",
            "QuoteMint11111111111111111111111111111111",
            0, // zero base reserve
            50_000_000_000,
            18446744073709551616,
            100.0,
            0.0,
            false,
        )
        .unwrap();
        provider.register_pool(empty_pool.clone());

        let req = LiquidityQuoteRequest::new(
            &empty_pool.pool_address,
            &empty_pool.quote_mint,
            &empty_pool.base_mint,
            1_000_000,
            50,
            TradeDirection::Buy,
        )
        .unwrap();

        let err = provider.get_quote(&req).await.unwrap_err();
        match err {
            DbcError::InvalidCurveState(msg) => {
                assert!(msg.contains("must be non-zero"));
            }
            other => panic!("Expected InvalidCurveState, got {:?}", other),
        }
    }
}
