//! Phase P4 Acceptance Tests: Live Solana RPC DBC Pool State and Pure Integer Quote Engine.
//!
//! Acceptance Criteria:
//! 1. Production quote generation requires successful retrieval of current on-chain pool state.
//! 2. RPC failure results in an error (NO QUOTE).
//! 3. No production quote can be generated from the old in-memory seeded state.
//! 4. Live values (reserves, price/state, pool status, relevant config, token mints) are verified against registry.
//! 5. Final monetary/execution authorization calculations strictly use pure integer arithmetic (no f64).

use solana_sdk::pubkey::Pubkey;
use std::time::Duration;

use equity_catalyst_api::engines::dbc_engine::{
    adapter::{
        decode_on_chain_dbc_pool, decode_on_chain_pool_config, encode_on_chain_dbc_pool,
        encode_on_chain_pool_config, DbcError, DecodedDbcPool, LiquidityProvider,
        MeteoraDbcProvider,
    },
    registry::{
        find_verified_pool_by_address, BACKED_NVDA_MINT, CANONICAL_DBC_CONFIG, MAINNET_USDC_MINT,
        METEORA_NVDA_USDC_POOL,
    },
};
use equity_catalyst_shared::{LiquidityQuoteRequest, PoolLiquidityState, TradeDirection};

#[tokio::test]
async fn test_production_dbc_pool_state_requires_live_rpc_and_fails_without_fallback() {
    // 1. Initialize production provider (mock_mode = false) pointing to an unavailable RPC endpoint
    let provider =
        MeteoraDbcProvider::new_with_timeout("http://127.0.0.1:18899", Duration::from_millis(150));
    assert!(
        !provider.is_mock_mode(),
        "Production provider must not be in mock mode"
    );

    let verified_pool = find_verified_pool_by_address(METEORA_NVDA_USDC_POOL)
        .expect("Canonical NVDA-USDC pool must exist in registry");

    let req = LiquidityQuoteRequest::new(
        verified_pool.pool_address,
        verified_pool.quote_mint,
        verified_pool.base_mint,
        5_000_000_000, // 5,000 USDC
        50,            // 0.5% slippage
        TradeDirection::Buy,
    )
    .expect("Valid request");

    // 2. When RPC fails: get_pool_state MUST return a structured RPC error
    let state_err = provider
        .get_pool_state(verified_pool.pool_address)
        .await
        .unwrap_err();
    match &state_err {
        DbcError::RpcError { pool, .. } | DbcError::RpcTimeout { pool, .. } => {
            assert_eq!(pool, verified_pool.pool_address);
        }
        other => panic!("Expected RpcError or RpcTimeout, got {:?}", other),
    }

    // 3. On RPC failure: NO QUOTE. Quote generation MUST fail immediately.
    // Zero fallback to seeded in-memory values!
    let quote_err = provider.get_quote(&req).await.unwrap_err();
    assert_eq!(
        quote_err, state_err,
        "Quote error must match RPC state retrieval error"
    );
}

#[tokio::test]
async fn test_unpermitted_pool_rejected_by_registry_guard() {
    let provider = MeteoraDbcProvider::new("https://api.mainnet-beta.solana.com");
    let unapproved_pool = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

    let err = provider.get_pool_state(unapproved_pool).await.unwrap_err();
    match err {
        DbcError::PoolNotPermitted(addr) => {
            assert_eq!(
                addr,
                format!(
                    "Pool '{}' is not registered in the canonical verified DBC pool registry",
                    unapproved_pool
                )
            );
        }
        other => panic!("Expected PoolNotPermitted error, got {:?}", other),
    }
}

#[test]
fn test_on_chain_dbc_virtual_pool_and_config_layout_decoding() {
    let pool_addr = METEORA_NVDA_USDC_POOL;
    let config_pubkey = Pubkey::new_unique();
    let creator_pubkey = Pubkey::new_unique();
    let base_mint_pubkey = Pubkey::new_unique();
    let base_vault_pubkey = Pubkey::new_unique();
    let quote_vault_pubkey = Pubkey::new_unique();

    let pool_state = DecodedDbcPool {
        config: config_pubkey,
        creator: creator_pubkey,
        base_mint: base_mint_pubkey,
        base_vault: base_vault_pubkey,
        quote_vault: quote_vault_pubkey,
        base_reserve: 75_000_000_000_000,
        quote_reserve: 180_000_000_000,
        sqrt_price: 20_000_000_000_000_000_000,
        activation_point: 500,
        pool_type: 0,
        is_migrated: false,
        migration_progress: 24,
    };

    let encoded_pool = encode_on_chain_dbc_pool(&pool_state);
    assert_eq!(
        encoded_pool.len(),
        424,
        "Meteora VirtualPool binary layout must be 424 bytes"
    );

    let decoded = decode_on_chain_dbc_pool(pool_addr, &encoded_pool)
        .expect("On-chain pool binary decoding must succeed");

    assert_eq!(decoded.config, config_pubkey);
    assert_eq!(decoded.base_mint, base_mint_pubkey);
    assert_eq!(decoded.base_reserve, 75_000_000_000_000);
    assert_eq!(decoded.quote_reserve, 180_000_000_000);
    assert_eq!(decoded.sqrt_price, 20_000_000_000_000_000_000);
    assert!(!decoded.is_migrated);
    assert_eq!(decoded.migration_progress, 24);

    // Test on-chain PoolConfig decoding
    let quote_mint = Pubkey::new_unique();
    let encoded_config = encode_on_chain_pool_config(&quote_mint);
    assert_eq!(
        encoded_config.len(),
        1048,
        "Meteora PoolConfig binary layout must be 1048 bytes"
    );

    let decoded_quote_mint = decode_on_chain_pool_config(CANONICAL_DBC_CONFIG, &encoded_config)
        .expect("On-chain pool config decoding must succeed");
    assert_eq!(decoded_quote_mint, quote_mint);
}

#[tokio::test]
async fn test_quote_calculations_use_live_state_and_pure_integer_arithmetic() {
    let provider = MeteoraDbcProvider::new_mock();
    let pool_address = "LiveOnChainPool1111111111111111111111111111";

    // Live state simulated from decoded Solana RPC account:
    // Base Reserve = 500_000_000_000 units (5,000 shares at 8 decimals)
    // Quote Reserve = 250_000_000_000 units (250,000 USDC at 6 decimals)
    let live_pool = PoolLiquidityState::new(
        pool_address,
        CANONICAL_DBC_CONFIG,
        BACKED_NVDA_MINT,
        MAINNET_USDC_MINT,
        500_000_000_000,
        250_000_000_000,
        18446744073709551616,
        120.0,
        15.0,
        false,
    )
    .unwrap();
    provider.register_pool(live_pool);

    let amount_in = 10_000_000_000u64; // 10,000 USDC
    let slippage_bps = 50u16; // 0.50%

    let req = LiquidityQuoteRequest::new(
        pool_address,
        MAINNET_USDC_MINT,
        BACKED_NVDA_MINT,
        amount_in,
        slippage_bps,
        TradeDirection::Buy,
    )
    .unwrap();

    let quote = provider
        .get_quote(&req)
        .await
        .expect("Quote generation succeeded");

    // Pure Integer Arithmetic Verification:
    // 1. Fee = 15 bps pool + 5 bps platform = 20 bps = (10_000_000_000 * 20) / 10_000 = 20_000_000
    assert_eq!(quote.fee_amount, 20_000_000);
    let _net_amount_in = amount_in - 20_000_000; // 9_980_000_000

    // 2. Expected Output (Constant Product AMM with u128 integer math):
    // numerator = 500_000_000_000 * 9_980_000_000 = 4_990_000_000_000_000_000_000
    // denominator = 250_000_000_000 + 9_980_000_000 = 259_980_000_000
    // expected_amount_out = 4_990_000_000_000_000_000_000 / 259_980_000_000 = 19_193_784_137
    assert_eq!(quote.expected_amount_out, 19_193_784_137);

    // 3. Minimum Output after 50 bps slippage:
    // min_amount_out = 19_193_784_137 * (10_000 - 50) / 10_000 = 19_097_815_216
    assert_eq!(quote.min_amount_out, 19_097_815_216);

    // 4. Exact ordering:
    assert!(quote.min_amount_out <= quote.expected_amount_out);
    assert_eq!(quote.amount_in, amount_in);
}
