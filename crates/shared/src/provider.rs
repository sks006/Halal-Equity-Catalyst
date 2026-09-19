//! Provider adapter abstractions and reference resolution for PreStocks, Tessera, Clawpump, and Backed Finance.

use serde::{Deserialize, Serialize};

use crate::asset::{
    verified_mainnet_assets, AssetProvider, AssetStatus, BACKED_AAPL_MINT, BACKED_NVDA_MINT,
    BACKED_SPYX_MINT, METEORA_AAPL_USDC_POOL, METEORA_NVDA_USDC_POOL, METEORA_SPYX_USDC_POOL,
};
use crate::shariah::ShariahStatus;

/// External provider reference linking an asset to a specific platform or protocol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderReference {
    pub provider: AssetProvider,
    pub source_id: String,
    pub underlying_symbol: String,
    pub description: String,
    pub status: AssetStatus,
}

/// Nature and authoritative status of an asset reference resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResolutionKind {
    /// Direct, exact match against a verified canonical asset in the primary registry.
    Exact,
    /// Authorized secondary reference explicitly registered on a verified canonical asset.
    SecondaryReference,
    /// Unofficial fallback mapping to underlying identity for lookup/pricing convenience only.
    ///
    /// # Safety Invariant
    /// Fallback resolution NEVER grants provider authorization, ownership verification,
    /// active listing approval, or Shariah eligibility. It always carries
    /// `shariah_status = Pending`, `ownership_verified = false`, and `is_executable = false`.
    FallbackIdentity,
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
    pub resolution_kind: ResolutionKind,
    pub is_fallback: bool,
    pub shariah_status: ShariahStatus,
    pub ownership_verified: bool,
    pub is_executable: bool,
}

impl ResolvedProviderAsset {
    /// Returns true if this resolved asset is fully verified and authorized for trade execution.
    #[inline]
    pub fn can_execute(&self) -> bool {
        self.is_executable
            && self.ownership_verified
            && self.shariah_status.is_approved()
            && self.resolution_kind != ResolutionKind::FallbackIdentity
    }

    /// Returns true if this resolution represents an authorized canonical or registered secondary provider reference.
    #[inline]
    pub fn is_authorized(&self) -> bool {
        self.resolution_kind != ResolutionKind::FallbackIdentity
    }
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
    /// # Fallback Hardening Rules:
    /// 1. Direct matches against verified mainnet assets are marked `ResolutionKind::Exact` with verified ownership.
    /// 2. Registered secondary references are marked `ResolutionKind::SecondaryReference` with verified ownership.
    /// 3. External provider prefixes without direct protocol authorization (e.g. "prestocks:NVDA", "clawpump:SPYX")
    ///    resolve identity only (`ResolutionKind::FallbackIdentity`). They carry `shariah_status = Pending`,
    ///    `ownership_verified = false`, and `is_executable = false`, barring them from execution.
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
                    resolution_kind: ResolutionKind::Exact,
                    is_fallback: false,
                    shariah_status: ShariahStatus::Approved,
                    ownership_verified: true,
                    is_executable: true,
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
                        resolution_kind: ResolutionKind::SecondaryReference,
                        is_fallback: false,
                        shariah_status: ShariahStatus::Approved,
                        ownership_verified: true,
                        is_executable: true,
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
                let canonical = Self::resolve_fallback_canonical(target_symbol)?;
                Some(ResolvedProviderAsset {
                    symbol: canonical.symbol,
                    name: canonical.name,
                    mint: canonical.mint,
                    decimals: canonical.decimals,
                    provider: canonical.provider,
                    source_id: canonical.source_id,
                    price_feed_id: canonical.price_feed_id,
                    meteora_pool: canonical.meteora_pool,
                    resolution_kind: ResolutionKind::FallbackIdentity,
                    is_fallback: true,
                    shariah_status: ShariahStatus::Pending,
                    ownership_verified: false,
                    is_executable: false,
                })
            }
        }
    }

    /// Resolves PreStocks pre-IPO equity references with on-chain identity fallback.
    /// Does NOT grant provider authorization, ownership verification, or execution approval.
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
            resolution_kind: ResolutionKind::FallbackIdentity,
            is_fallback: true,
            shariah_status: ShariahStatus::Pending,
            ownership_verified: false,
            is_executable: false,
        })
    }

    /// Resolves Tessera fractionalized vault share references.
    /// Unregistered secondary references fall back to identity only.
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
            resolution_kind: ResolutionKind::FallbackIdentity,
            is_fallback: true,
            shariah_status: ShariahStatus::Pending,
            ownership_verified: false,
            is_executable: false,
        })
    }

    /// Resolves Clawpump bonding curve references to underlying identity.
    /// Does NOT grant provider authorization or execution approval.
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
            resolution_kind: ResolutionKind::FallbackIdentity,
            is_fallback: true,
            shariah_status: ShariahStatus::Pending,
            ownership_verified: false,
            is_executable: false,
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
            resolution_kind: ResolutionKind::Exact,
            is_fallback: false,
            shariah_status: ShariahStatus::Approved,
            ownership_verified: true,
            is_executable: true,
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
                resolution_kind: ResolutionKind::Exact,
                is_fallback: false,
                shariah_status: ShariahStatus::Approved,
                ownership_verified: true,
                is_executable: true,
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
                resolution_kind: ResolutionKind::Exact,
                is_fallback: false,
                shariah_status: ShariahStatus::Approved,
                ownership_verified: true,
                is_executable: true,
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
                resolution_kind: ResolutionKind::Exact,
                is_fallback: false,
                shariah_status: ShariahStatus::Approved,
                ownership_verified: true,
                is_executable: true,
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
        // PreStocks resolution - identity fallback only
        let prestocks_nvda =
            ProviderResolver::resolve("prestocks:NVDA").expect("Resolves PreStocks NVDA");
        assert_eq!(prestocks_nvda.symbol, "NVDA");
        assert_eq!(prestocks_nvda.mint, BACKED_NVDA_MINT);
        assert_eq!(prestocks_nvda.provider, AssetProvider::PreStocks);
        assert!(prestocks_nvda.is_fallback);
        assert_eq!(
            prestocks_nvda.resolution_kind,
            ResolutionKind::FallbackIdentity
        );
        assert_eq!(prestocks_nvda.shariah_status, ShariahStatus::Pending);
        assert!(!prestocks_nvda.ownership_verified);
        assert!(!prestocks_nvda.is_executable);
        assert!(!prestocks_nvda.can_execute());
        assert!(!prestocks_nvda.is_authorized());

        // Tessera registered secondary reference - authorized on verified asset
        let tessera_aapl =
            ProviderResolver::resolve("tessera:AAPL").expect("Resolves Tessera AAPL");
        assert_eq!(tessera_aapl.symbol, "AAPL");
        assert_eq!(tessera_aapl.mint, BACKED_AAPL_MINT);
        assert_eq!(tessera_aapl.provider, AssetProvider::Tessera);
        assert!(!tessera_aapl.is_fallback); // Explicitly registered in canonical asset registry
        assert_eq!(
            tessera_aapl.resolution_kind,
            ResolutionKind::SecondaryReference
        );
        assert_eq!(tessera_aapl.shariah_status, ShariahStatus::Approved);
        assert!(tessera_aapl.ownership_verified);
        assert!(tessera_aapl.is_executable);
        assert!(tessera_aapl.can_execute());
        assert!(tessera_aapl.is_authorized());

        // Tessera unmapped secondary reference fallback
        let tessera_spy = ProviderResolver::resolve("tessera:SPY").expect("Resolves Tessera SPY");
        assert_eq!(tessera_spy.symbol, "SPYx");
        assert_eq!(tessera_spy.mint, BACKED_SPYX_MINT);
        assert_eq!(tessera_spy.provider, AssetProvider::Tessera);
        assert!(tessera_spy.is_fallback);
        assert_eq!(
            tessera_spy.resolution_kind,
            ResolutionKind::FallbackIdentity
        );
        assert_eq!(tessera_spy.shariah_status, ShariahStatus::Pending);
        assert!(!tessera_spy.ownership_verified);
        assert!(!tessera_spy.can_execute());

        // Clawpump resolution
        let clawpump_spy =
            ProviderResolver::resolve("clawpump:SPY").expect("Resolves Clawpump SPY");
        assert_eq!(clawpump_spy.symbol, "SPYx");
        assert_eq!(clawpump_spy.mint, BACKED_SPYX_MINT);
        assert_eq!(clawpump_spy.provider, AssetProvider::Clawpump);
        assert!(clawpump_spy.is_fallback);
        assert_eq!(
            clawpump_spy.resolution_kind,
            ResolutionKind::FallbackIdentity
        );
        assert_eq!(clawpump_spy.shariah_status, ShariahStatus::Pending);
        assert!(!clawpump_spy.ownership_verified);
        assert!(!clawpump_spy.can_execute());
    }

    #[test]
    fn test_fallback_cannot_execute_or_claim_shariah_eligibility() {
        let fallback_asset =
            ProviderResolver::resolve("prestocks:NVDA").expect("Resolves fallback");
        assert_eq!(
            fallback_asset.resolution_kind,
            ResolutionKind::FallbackIdentity
        );
        assert_eq!(fallback_asset.shariah_status, ShariahStatus::Pending);
        assert!(!fallback_asset.ownership_verified);
        assert!(!fallback_asset.can_execute());
    }

    #[test]
    fn test_resolve_direct_and_mint_lookups() {
        // Direct symbol
        let direct_nvda = ProviderResolver::resolve("NVDA").expect("Direct NVDA");
        assert_eq!(direct_nvda.symbol, "NVDA");
        assert_eq!(direct_nvda.mint, BACKED_NVDA_MINT);
        assert_eq!(direct_nvda.decimals, 8);
        assert_eq!(direct_nvda.resolution_kind, ResolutionKind::Exact);
        assert_eq!(direct_nvda.shariah_status, ShariahStatus::Approved);
        assert!(direct_nvda.ownership_verified);
        assert!(direct_nvda.can_execute());

        // Direct mint lookup
        let mint_lookup = ProviderResolver::resolve(BACKED_AAPL_MINT).expect("Mint AAPL");
        assert_eq!(mint_lookup.symbol, "AAPL");
        assert_eq!(mint_lookup.mint, BACKED_AAPL_MINT);
        assert_eq!(mint_lookup.resolution_kind, ResolutionKind::Exact);
        assert_eq!(mint_lookup.shariah_status, ShariahStatus::Approved);
        assert!(mint_lookup.ownership_verified);
        assert!(mint_lookup.can_execute());
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
