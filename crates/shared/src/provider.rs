//! Provider adapter abstractions and reference resolution for PreStocks, Tessera, Clawpump, and Backed Finance.

use serde::{Deserialize, Serialize};

use crate::asset::{
    verified_mainnet_assets, AssetProvider, AssetStatus, BACKED_AAPL_MINT, BACKED_NVDA_MINT,
    BACKED_SPYX_MINT, METEORA_AAPL_USDC_POOL, METEORA_NVDA_USDC_POOL, METEORA_SPYX_USDC_POOL,
};

/// External provider reference linking an asset to a specific platform or protocol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderReference {
    pub provider: AssetProvider,
    pub source_id: String,
    pub underlying_symbol: String,
    pub description: String,
    pub status: AssetStatus,
}

/// Normalized asset representation resolved from one or more provider integrations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedProviderAsset {
    pub symbol: String,
    pub name: String,
    pub mint: String,
    pub decimals: u8,
    pub provider: AssetProvider,
    pub source_id: String,
    pub price_feed_id: String,
    pub meteora_pool: Option<String>,
    pub is_fallback: bool,
}

/// Provider resolver providing unified resolution and graceful on-chain fallback across external protocols.
pub struct ProviderResolver;

impl ProviderResolver {
    /// Returns the complete list of supported tokenized equity providers.
    pub fn list_supported_providers() -> Vec<AssetProvider> {
        vec![
            AssetProvider::PreStocks,
            AssetProvider::Tessera,
            AssetProvider::Clawpump,
            AssetProvider::Backed,
            AssetProvider::Pyth,
            AssetProvider::Native,
        ]
    }

    /// Resolves any provider reference, ticker symbol, or token mint to its normalized domain asset.
    ///
    /// Fallback Rule:
    /// If an external provider reference (e.g. "prestocks:NVDA", "tessera:AAPL", "clawpump:SPYX")
    /// is queried, this method maps it to the statutory tokenized equity mint on Solana,
    /// backed by statutory certificates, canonical Pyth Hermes oracle price feeds, and verified Meteora DBC pools.
    pub fn resolve(query: &str) -> Option<ResolvedProviderAsset> {
        let q = query.trim();
        if q.is_empty() {
            return None;
        }

        // 1. Direct match against verified mainnet assets
        let verified = verified_mainnet_assets();
        for asset in &verified {
            if asset.symbol().eq_ignore_ascii_case(q)
                || asset.id().eq_ignore_ascii_case(q)
                || asset.mint().eq_ignore_ascii_case(q)
            {
                return Some(ResolvedProviderAsset {
                    symbol: asset.symbol().to_string(),
                    name: asset.name().to_string(),
                    mint: asset.mint().to_string(),
                    decimals: asset.decimals(),
                    provider: asset.provider.provider,
                    source_id: asset.id().to_string(),
                    price_feed_id: asset.price_feed_id().to_string(),
                    meteora_pool: asset.provider.meteora_pool.clone(),
                    is_fallback: false,
                });
            }

            // Check secondary references
            if let Some(ref sec) = asset.provider.secondary_reference {
                if sec.eq_ignore_ascii_case(q) {
                    return Some(ResolvedProviderAsset {
                        symbol: asset.symbol().to_string(),
                        name: asset.name().to_string(),
                        mint: asset.mint().to_string(),
                        decimals: asset.decimals(),
                        provider: AssetProvider::Tessera,
                        source_id: sec.clone(),
                        price_feed_id: asset.price_feed_id().to_string(),
                        meteora_pool: asset.provider.meteora_pool.clone(),
                        is_fallback: false,
                    });
                }
            }
        }

        // 2. Parse provider prefix syntax: "provider:symbol"
        let (prefix, symbol) = match q.split_once(':') {
            Some((p, s)) => (p.trim().to_lowercase(), s.trim().to_uppercase()),
            None => (String::new(), q.to_uppercase()),
        };

        let target_symbol = symbol.trim_end_matches('X').trim_end_matches('x');

        match prefix.as_str() {
            "prestocks" => Self::resolve_prestocks_reference(&symbol, target_symbol),
            "tessera" => Self::resolve_tessera_reference(&symbol, target_symbol),
            "clawpump" => Self::resolve_clawpump_reference(&symbol, target_symbol),
            "backed" => Self::resolve_backed_reference(&symbol, target_symbol),
            _ => {
                // Fallback attempt with normalized symbol
                Self::resolve_fallback_canonical(target_symbol)
            }
        }
    }

    /// Resolves PreStocks pre-IPO equity references with on-chain statutory fallback.
    fn resolve_prestocks_reference(
        source_symbol: &str,
        base_symbol: &str,
    ) -> Option<ResolvedProviderAsset> {
        let canonical = Self::resolve_fallback_canonical(base_symbol)?;
        Some(ResolvedProviderAsset {
            symbol: canonical.symbol,
            name: format!("PreStocks {}", canonical.name),
            mint: canonical.mint,
            decimals: canonical.decimals,
            provider: AssetProvider::PreStocks,
            source_id: format!("prestocks:{}", source_symbol),
            price_feed_id: canonical.price_feed_id,
            meteora_pool: canonical.meteora_pool,
            is_fallback: true,
        })
    }

    /// Resolves Tessera fractionalized vault share references with on-chain statutory fallback.
    fn resolve_tessera_reference(
        source_symbol: &str,
        base_symbol: &str,
    ) -> Option<ResolvedProviderAsset> {
        let canonical = Self::resolve_fallback_canonical(base_symbol)?;
        Some(ResolvedProviderAsset {
            symbol: canonical.symbol,
            name: format!("Tessera Fractional {}", canonical.name),
            mint: canonical.mint,
            decimals: canonical.decimals,
            provider: AssetProvider::Tessera,
            source_id: format!("tessera:{}", source_symbol),
            price_feed_id: canonical.price_feed_id,
            meteora_pool: canonical.meteora_pool,
            is_fallback: true,
        })
    }

    /// Resolves Clawpump bonding curve references with fair-value Pyth oracle anchoring.
    fn resolve_clawpump_reference(
        source_symbol: &str,
        base_symbol: &str,
    ) -> Option<ResolvedProviderAsset> {
        let canonical = Self::resolve_fallback_canonical(base_symbol)?;
        Some(ResolvedProviderAsset {
            symbol: canonical.symbol,
            name: format!("Clawpump DBC {}", canonical.name),
            mint: canonical.mint,
            decimals: canonical.decimals,
            provider: AssetProvider::Clawpump,
            source_id: format!("clawpump:{}", source_symbol),
            price_feed_id: canonical.price_feed_id,
            meteora_pool: canonical.meteora_pool,
            is_fallback: true,
        })
    }

    /// Resolves Backed Finance canonical statutory stock references.
    fn resolve_backed_reference(
        source_symbol: &str,
        base_symbol: &str,
    ) -> Option<ResolvedProviderAsset> {
        let canonical = Self::resolve_fallback_canonical(base_symbol)?;
        Some(ResolvedProviderAsset {
            symbol: canonical.symbol,
            name: canonical.name,
            mint: canonical.mint,
            decimals: canonical.decimals,
            provider: AssetProvider::Backed,
            source_id: format!("backed:{}", source_symbol),
            price_feed_id: canonical.price_feed_id,
            meteora_pool: canonical.meteora_pool,
            is_fallback: false,
        })
    }

    /// Canonical on-chain fallback mapping based on verified statutory assets.
    fn resolve_fallback_canonical(base_symbol: &str) -> Option<ResolvedProviderAsset> {
        match base_symbol.to_uppercase().as_str() {
            "NVDA" => Some(ResolvedProviderAsset {
                symbol: "NVDA".to_string(),
                name: "Backed NVIDIA".to_string(),
                mint: BACKED_NVDA_MINT.to_string(),
                decimals: 8,
                provider: AssetProvider::Backed,
                source_id: "backed:NVDAx".to_string(),
                price_feed_id: "3155e714652285e6834d8ef0b3558163f4585c5b9679f222956cf57fb3645391"
                    .to_string(),
                meteora_pool: Some(METEORA_NVDA_USDC_POOL.to_string()),
                is_fallback: false,
            }),
            "AAPL" => Some(ResolvedProviderAsset {
                symbol: "AAPL".to_string(),
                name: "Backed Apple".to_string(),
                mint: BACKED_AAPL_MINT.to_string(),
                decimals: 8,
                provider: AssetProvider::Backed,
                source_id: "backed:AAPLx".to_string(),
                price_feed_id: "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175ec697df854ab"
                    .to_string(),
                meteora_pool: Some(METEORA_AAPL_USDC_POOL.to_string()),
                is_fallback: false,
            }),
            "SPY" | "SPYX" => Some(ResolvedProviderAsset {
                symbol: "SPYx".to_string(),
                name: "Backed S&P 500 Index".to_string(),
                mint: BACKED_SPYX_MINT.to_string(),
                decimals: 8,
                provider: AssetProvider::Backed,
                source_id: "backed:SPYx".to_string(),
                price_feed_id: "2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b"
                    .to_string(),
                meteora_pool: Some(METEORA_SPYX_USDC_POOL.to_string()),
                is_fallback: false,
            }),
            _ => None,
        }
    }

    /// Lists all active external provider references for a given canonical symbol.
    pub fn list_provider_references_for_asset(symbol: &str) -> Vec<ProviderReference> {
        let sym = symbol.to_uppercase();
        let base = sym.trim_end_matches('X');

        vec![
            ProviderReference {
                provider: AssetProvider::Backed,
                source_id: format!("backed:{}x", base),
                underlying_symbol: base.to_string(),
                description: format!(
                    "Statutory Tokenized Security under Swiss DLT Act ({})",
                    base
                ),
                status: AssetStatus::Active,
            },
            ProviderReference {
                provider: AssetProvider::PreStocks,
                source_id: format!("prestocks:{}", base),
                underlying_symbol: base.to_string(),
                description: format!(
                    "PreStocks tokenized private equity representation for {}",
                    base
                ),
                status: AssetStatus::Active,
            },
            ProviderReference {
                provider: AssetProvider::Tessera,
                source_id: format!("tessera:{}", base),
                underlying_symbol: base.to_string(),
                description: format!("Tessera fractional collective ownership vault for {}", base),
                status: AssetStatus::Active,
            },
            ProviderReference {
                provider: AssetProvider::Clawpump,
                source_id: format!("clawpump:{}", base),
                underlying_symbol: base.to_string(),
                description: format!(
                    "Meteora Dynamic Bonding Curve launchpad market for {}",
                    base
                ),
                status: AssetStatus::Active,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_supported_providers() {
        let providers = ProviderResolver::list_supported_providers();
        assert!(providers.contains(&AssetProvider::PreStocks));
        assert!(providers.contains(&AssetProvider::Tessera));
        assert!(providers.contains(&AssetProvider::Clawpump));
        assert!(providers.contains(&AssetProvider::Backed));
        assert!(providers.contains(&AssetProvider::Pyth));
        assert!(providers.contains(&AssetProvider::Native));
    }

    #[test]
    fn test_resolve_prestocks_and_tessera_references() {
        // PreStocks resolution
        let prestocks_nvda =
            ProviderResolver::resolve("prestocks:NVDA").expect("Resolves PreStocks NVDA");
        assert_eq!(prestocks_nvda.symbol, "NVDA");
        assert_eq!(prestocks_nvda.mint, BACKED_NVDA_MINT);
        assert_eq!(prestocks_nvda.provider, AssetProvider::PreStocks);
        assert!(prestocks_nvda.is_fallback);

        // Tessera registered secondary reference
        let tessera_aapl =
            ProviderResolver::resolve("tessera:AAPL").expect("Resolves Tessera AAPL");
        assert_eq!(tessera_aapl.symbol, "AAPL");
        assert_eq!(tessera_aapl.mint, BACKED_AAPL_MINT);
        assert_eq!(tessera_aapl.provider, AssetProvider::Tessera);
        assert!(!tessera_aapl.is_fallback); // Explicitly registered in canonical asset registry

        // Tessera unmapped secondary reference fallback
        let tessera_spy = ProviderResolver::resolve("tessera:SPY").expect("Resolves Tessera SPY");
        assert_eq!(tessera_spy.symbol, "SPYx");
        assert_eq!(tessera_spy.mint, BACKED_SPYX_MINT);
        assert_eq!(tessera_spy.provider, AssetProvider::Tessera);
        assert!(tessera_spy.is_fallback);

        // Clawpump resolution
        let clawpump_spy =
            ProviderResolver::resolve("clawpump:SPY").expect("Resolves Clawpump SPY");
        assert_eq!(clawpump_spy.symbol, "SPYx");
        assert_eq!(clawpump_spy.mint, BACKED_SPYX_MINT);
        assert_eq!(clawpump_spy.provider, AssetProvider::Clawpump);
        assert!(clawpump_spy.is_fallback);
    }

    #[test]
    fn test_resolve_direct_and_mint_lookups() {
        // Direct symbol
        let direct_nvda = ProviderResolver::resolve("NVDA").expect("Direct NVDA");
        assert_eq!(direct_nvda.symbol, "NVDA");
        assert_eq!(direct_nvda.mint, BACKED_NVDA_MINT);
        assert_eq!(direct_nvda.decimals, 8);

        // Direct mint lookup
        let mint_lookup = ProviderResolver::resolve(BACKED_AAPL_MINT).expect("Mint AAPL");
        assert_eq!(mint_lookup.symbol, "AAPL");
        assert_eq!(mint_lookup.mint, BACKED_AAPL_MINT);
    }

    #[test]
    fn test_resolve_unknown_returns_none() {
        assert!(ProviderResolver::resolve("unknown:FAKE_STOCK").is_none());
        assert!(ProviderResolver::resolve("").is_none());
        assert!(ProviderResolver::resolve("   ").is_none());
    }

    #[test]
    fn test_list_provider_references_for_asset() {
        let refs = ProviderResolver::list_provider_references_for_asset("NVDA");
        assert_eq!(refs.len(), 4);
        assert!(refs.iter().any(|r| r.provider == AssetProvider::Backed));
        assert!(refs.iter().any(|r| r.provider == AssetProvider::PreStocks));
        assert!(refs.iter().any(|r| r.provider == AssetProvider::Tessera));
        assert!(refs.iter().any(|r| r.provider == AssetProvider::Clawpump));
    }
}
