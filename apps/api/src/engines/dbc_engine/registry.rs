//! Canonical registry of verified Meteora DBC pools for tokenized equities.

use equity_catalyst_shared::PoolLiquidityState;

use crate::engines::dbc_engine::pricing::compute_sqrt_price_q64;

/// Canonical Quote Token Mints on Solana.
pub const MAINNET_USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const DEVNET_USDC_MINT: &str = "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr";
pub const WRAPPED_SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Canonical Backed Tokenized Equity Mints on Solana Mainnet.
pub const BACKED_NVDA_MINT: &str = "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh";
pub const BACKED_AAPL_MINT: &str = "XsbEhLAtcf6HdfpFZ5xEMdqW8nfAvcsP5bdudRLJzJp";
pub const BACKED_SPYX_MINT: &str = "XsoCS1TfEyfFhfvj8EtZ528L3CaKBDBRqRapnBbDF2W";

/// Canonical Meteora DBC Config Account for Equity Discovery Curves.
pub const CANONICAL_DBC_CONFIG: &str = "ASv4E2yuTiE5rsWnUG5LWvVYVQ8UnakiYTwgHtz5pf6b";

/// Verified Meteora DBC Pool PDAs for Supported Tokenized Equities.
pub const METEORA_NVDA_USDC_POOL: &str = "JCqWLp5RAaC3FPFX8MoAt7yRuPqRW2W9ZG3Byigxd1A7";
pub const METEORA_NVDA_SOL_POOL: &str = "G8RrQbHii2bqRUJg3MvSdkumNU6Kx2xZMHFrefGv3NQY";
pub const METEORA_AAPL_USDC_POOL: &str = "C3Zm5CTFQxfCdbbDameKXRsmMUz8nfpkHqrevmdX97YS";
pub const METEORA_AAPL_SOL_POOL: &str = "5Hh5PPeNw65eeJnz9UCzzxEnSrNP7iPrzti5uhVNECKr";
pub const METEORA_SPYX_USDC_POOL: &str = "CNutHtA6JUuwwWGCcJXobusHdRWZ4EgJTSUxxzXqnRj7";
pub const METEORA_SPYX_SOL_POOL: &str = "2zF6y56rn6LBeiY6n9Hqk1CVS5o663gSp6r16nDKpPZg";
pub const METEORA_DEVNET_NVDA_USDC_POOL: &str = "8QByFpYZdnH1jPgL3dQhi7nYrKbYkziiLvWyTaHnE5ff";

/// Verified pool metadata and curve state parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedPoolInfo {
    pub pool_address: &'static str,
    pub config_address: &'static str,
    pub base_mint: &'static str,
    pub quote_mint: &'static str,
    pub symbol: &'static str,
    pub quote_symbol: &'static str,
    pub base_decimals: u8,
    pub quote_decimals: u8,
    pub initial_price_usd: f64,
    pub current_price_usd: f64,
    pub curve_profile: &'static str,
    pub graduation_threshold: f64,
    pub initial_base_reserve: u64,
    pub initial_quote_reserve: u64,
    pub curve_progress_pct: f64,
    pub is_migrated: bool,
}

/// Returns the complete registry of verified Meteora DBC pools.
pub fn verified_dbc_pools() -> Vec<VerifiedPoolInfo> {
    vec![
        VerifiedPoolInfo {
            pool_address: METEORA_NVDA_USDC_POOL,
            config_address: CANONICAL_DBC_CONFIG,
            base_mint: BACKED_NVDA_MINT,
            quote_mint: MAINNET_USDC_MINT,
            symbol: "NVDAx",
            quote_symbol: "USDC",
            base_decimals: 8,
            quote_decimals: 6,
            initial_price_usd: 100.73,
            current_price_usd: 118.50,
            curve_profile: "equity_discovery",
            graduation_threshold: 100_000.0,
            initial_base_reserve: 80_000_000_000_000, // 800k shares (8 decimals)
            initial_quote_reserve: 150_000_000_000,   // 150k USDC (6 decimals)
            curve_progress_pct: 18.5,
            is_migrated: false,
        },
        VerifiedPoolInfo {
            pool_address: METEORA_NVDA_SOL_POOL,
            config_address: CANONICAL_DBC_CONFIG,
            base_mint: BACKED_NVDA_MINT,
            quote_mint: WRAPPED_SOL_MINT,
            symbol: "NVDAx",
            quote_symbol: "SOL",
            base_decimals: 8,
            quote_decimals: 9,
            initial_price_usd: 100.73,
            current_price_usd: 118.50,
            curve_profile: "equity_discovery",
            graduation_threshold: 750.0,
            initial_base_reserve: 40_000_000_000_000, // 400k shares
            initial_quote_reserve: 1_200_000_000_000, // 1,200 SOL (9 decimals)
            curve_progress_pct: 12.0,
            is_migrated: false,
        },
        VerifiedPoolInfo {
            pool_address: METEORA_AAPL_USDC_POOL,
            config_address: CANONICAL_DBC_CONFIG,
            base_mint: BACKED_AAPL_MINT,
            quote_mint: MAINNET_USDC_MINT,
            symbol: "AAPLx",
            quote_symbol: "USDC",
            base_decimals: 8,
            quote_decimals: 6,
            initial_price_usd: 190.50,
            current_price_usd: 224.20,
            curve_profile: "equity_discovery",
            graduation_threshold: 100_000.0,
            initial_base_reserve: 50_000_000_000_000, // 500k shares
            initial_quote_reserve: 200_000_000_000,   // 200k USDC
            curve_progress_pct: 22.0,
            is_migrated: false,
        },
        VerifiedPoolInfo {
            pool_address: METEORA_AAPL_SOL_POOL,
            config_address: CANONICAL_DBC_CONFIG,
            base_mint: BACKED_AAPL_MINT,
            quote_mint: WRAPPED_SOL_MINT,
            symbol: "AAPLx",
            quote_symbol: "SOL",
            base_decimals: 8,
            quote_decimals: 9,
            initial_price_usd: 190.50,
            current_price_usd: 224.20,
            curve_profile: "equity_discovery",
            graduation_threshold: 750.0,
            initial_base_reserve: 30_000_000_000_000, // 300k shares
            initial_quote_reserve: 1_500_000_000_000, // 1,500 SOL
            curve_progress_pct: 15.4,
            is_migrated: false,
        },
        VerifiedPoolInfo {
            pool_address: METEORA_SPYX_USDC_POOL,
            config_address: CANONICAL_DBC_CONFIG,
            base_mint: BACKED_SPYX_MINT,
            quote_mint: MAINNET_USDC_MINT,
            symbol: "SPYx",
            quote_symbol: "USDC",
            base_decimals: 8,
            quote_decimals: 6,
            initial_price_usd: 480.00,
            current_price_usd: 565.00,
            curve_profile: "equity_discovery",
            graduation_threshold: 200_000.0,
            initial_base_reserve: 20_000_000_000_000, // 200k shares
            initial_quote_reserve: 250_000_000_000,   // 250k USDC
            curve_progress_pct: 28.0,
            is_migrated: false,
        },
        VerifiedPoolInfo {
            pool_address: METEORA_SPYX_SOL_POOL,
            config_address: CANONICAL_DBC_CONFIG,
            base_mint: BACKED_SPYX_MINT,
            quote_mint: WRAPPED_SOL_MINT,
            symbol: "SPYx",
            quote_symbol: "SOL",
            base_decimals: 8,
            quote_decimals: 9,
            initial_price_usd: 480.00,
            current_price_usd: 565.00,
            curve_profile: "equity_discovery",
            graduation_threshold: 1_500.0,
            initial_base_reserve: 15_000_000_000_000, // 150k shares
            initial_quote_reserve: 2_000_000_000_000, // 2,000 SOL
            curve_progress_pct: 19.8,
            is_migrated: false,
        },
        VerifiedPoolInfo {
            pool_address: METEORA_DEVNET_NVDA_USDC_POOL,
            config_address: CANONICAL_DBC_CONFIG,
            base_mint: BACKED_NVDA_MINT,
            quote_mint: DEVNET_USDC_MINT,
            symbol: "NVDAx",
            quote_symbol: "USDC",
            base_decimals: 8,
            quote_decimals: 6,
            initial_price_usd: 100.73,
            current_price_usd: 118.50,
            curve_profile: "equity_discovery",
            graduation_threshold: 100_000.0,
            initial_base_reserve: 80_000_000_000_000,
            initial_quote_reserve: 150_000_000_000,
            curve_progress_pct: 10.0,
            is_migrated: false,
        },
    ]
}

/// Finds a verified pool by base and quote mints.
pub fn find_verified_pool_by_mints(base_mint: &str, quote_mint: &str) -> Option<VerifiedPoolInfo> {
    verified_dbc_pools().into_iter().find(|p| {
        (p.base_mint == base_mint && p.quote_mint == quote_mint)
            || (p.base_mint == quote_mint && p.quote_mint == base_mint)
    })
}

/// Finds a verified pool by its Solana account address.
pub fn find_verified_pool_by_address(pool_address: &str) -> Option<VerifiedPoolInfo> {
    verified_dbc_pools()
        .into_iter()
        .find(|p| p.pool_address == pool_address)
}

/// Builds an initial `PoolLiquidityState` for a verified pool with computed Q64.64 sqrt price.
///
/// NOTE: Strictly for test fixtures and isolated offline mocks.
/// In production, live pool state (reserves, sqrt_price, curve progress, status)
/// MUST be retrieved directly from Solana on-chain RPC accounts.
pub fn build_initial_pool_liquidity_state(pool: &VerifiedPoolInfo) -> PoolLiquidityState {
    let sqrt_price_q64 = compute_sqrt_price_q64(
        pool.current_price_usd,
        pool.base_decimals,
        pool.quote_decimals,
    );

    PoolLiquidityState::new(
        pool.pool_address,
        pool.config_address,
        pool.base_mint,
        pool.quote_mint,
        pool.initial_base_reserve,
        pool.initial_quote_reserve,
        sqrt_price_q64,
        pool.current_price_usd,
        pool.curve_progress_pct,
        pool.is_migrated,
    )
    .expect("Valid pool state from verified pool info")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verified_dbc_pools_coverage() {
        let pools = verified_dbc_pools();
        assert_eq!(pools.len(), 7);

        // Verify all assets have USDC and SOL pairs
        let symbols = ["NVDAx", "AAPLx", "SPYx"];
        for symbol in &symbols {
            let usdc_pool = pools.iter().find(|p| {
                p.symbol == *symbol && p.quote_symbol == "USDC" && p.quote_mint == MAINNET_USDC_MINT
            });
            assert!(
                usdc_pool.is_some(),
                "Missing mainnet USDC pool for {}",
                symbol
            );

            let sol_pool = pools
                .iter()
                .find(|p| p.symbol == *symbol && p.quote_symbol == "SOL");
            assert!(sol_pool.is_some(), "Missing SOL pool for {}", symbol);
        }

        // Verify lookup by mints
        let nvda_usdc = find_verified_pool_by_mints(BACKED_NVDA_MINT, MAINNET_USDC_MINT);
        assert!(nvda_usdc.is_some());
        assert_eq!(nvda_usdc.unwrap().pool_address, METEORA_NVDA_USDC_POOL);

        // Verify reverse lookup
        let nvda_usdc_rev = find_verified_pool_by_mints(MAINNET_USDC_MINT, BACKED_NVDA_MINT);
        assert!(nvda_usdc_rev.is_some());
        assert_eq!(nvda_usdc_rev.unwrap().pool_address, METEORA_NVDA_USDC_POOL);

        // Verify lookup by address
        let pool_by_addr = find_verified_pool_by_address(METEORA_AAPL_USDC_POOL);
        assert!(pool_by_addr.is_some());
        assert_eq!(pool_by_addr.unwrap().symbol, "AAPLx");
    }

    #[test]
    fn test_initial_pool_liquidity_state_math() {
        let pools = verified_dbc_pools();
        for pool in &pools {
            let state = build_initial_pool_liquidity_state(pool);
            assert_eq!(state.pool_address, pool.pool_address);
            assert_eq!(state.base_mint, pool.base_mint);
            assert_eq!(state.quote_mint, pool.quote_mint);
            assert!(state.base_reserve > 0);
            assert!(state.quote_reserve > 0);
            assert!(state.sqrt_price_q64 > 0);
            assert_eq!(state.current_price_usd, pool.current_price_usd);
            assert!(!state.is_migrated);
        }
    }
}
