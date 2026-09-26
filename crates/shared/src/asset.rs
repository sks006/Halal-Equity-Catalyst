//! Canonical Asset Identity and Tokenized Equity Domain Model.
//!
//! Provides pure, deterministic representations of stocks, tokenized RWAs,
//! and clean separation between static asset identity and dynamic market data.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::validation::ValidationError;

/// Supported asset classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Stock,
    Etf,
    Commodity,
    Crypto,
    Index,
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stock => write!(f, "stock"),
            Self::Etf => write!(f, "etf"),
            Self::Commodity => write!(f, "commodity"),
            Self::Crypto => write!(f, "crypto"),
            Self::Index => write!(f, "index"),
        }
    }
}

/// Tokenized equity asset provider / issuer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetProvider {
    PreStocks,
    Tessera,
    Clawpump,
    Backed,
    Pyth,
    Native,
}

impl fmt::Display for AssetProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreStocks => write!(f, "prestocks"),
            Self::Tessera => write!(f, "tessera"),
            Self::Clawpump => write!(f, "clawpump"),
            Self::Backed => write!(f, "backed"),
            Self::Pyth => write!(f, "pyth"),
            Self::Native => write!(f, "native"),
        }
    }
}

/// Target blockchain deployment network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Network {
    SolanaMainnet,
    SolanaDevnet,
    SolanaTestnet,
    Localnet,
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SolanaMainnet => write!(f, "solana-mainnet"),
            Self::SolanaDevnet => write!(f, "solana-devnet"),
            Self::SolanaTestnet => write!(f, "solana-testnet"),
            Self::Localnet => write!(f, "localnet"),
        }
    }
}

/// Operational status of an asset in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    Active,
    Halted,
    Delisted,
    Pending,
}

impl fmt::Display for AssetStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Halted => write!(f, "halted"),
            Self::Delisted => write!(f, "delisted"),
            Self::Pending => write!(f, "pending"),
        }
    }
}

/// Static, immutable identity of an asset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetIdentity {
    /// Unique canonical identifier, e.g. "prestocks:NVDA" or "tessera:AAPL"
    pub asset_id: String,
    /// Trading ticker symbol, e.g. "NVDA", "AAPL", "USDC"
    pub symbol: String,
    /// Human-readable legal or commercial name
    pub name: String,
    /// Asset class / type
    pub asset_type: AssetType,
    /// Standard underlying instrument identifier (e.g. ISIN, CUSIP, Bloomberg, or Exchange ticker)
    pub underlying_reference: String,
}

/// On-chain token specifications.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenDetails {
    /// Solana SPL Token mint public key (base58)
    pub mint: String,
    /// Token fractional decimal places (e.g. 6 for USDC/stock tokens, 9 for SOL)
    pub decimals: u8,
    /// Blockchain network
    pub network: Network,
}

/// Protocol provider and market integration parameters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Primary asset provider / tokenization platform
    pub provider: AssetProvider,
    /// Pyth price feed hex identifier
    pub price_feed_id: String,
    /// Meteora DBC pool account address (if initialized)
    pub meteora_pool: Option<String>,
    /// Secondary provider reference (e.g. Tessera reference for PreStocks asset)
    pub secondary_reference: Option<String>,
    /// Current operational status
    pub status: AssetStatus,
}

/// Canonical composite Asset representation cleanly separating identity,
/// token properties, and provider integrations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Asset {
    pub identity: AssetIdentity,
    pub token: TokenDetails,
    pub provider: ProviderConfig,
}

impl Asset {
    pub fn new(
        identity: AssetIdentity,
        token: TokenDetails,
        provider: ProviderConfig,
    ) -> Result<Self, ValidationError> {
        if identity.symbol.trim().is_empty() {
            return Err(ValidationError::EmptySymbol);
        }
        if identity.asset_id.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "asset_id cannot be empty".to_string(),
            ));
        }
        if token.mint.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "token mint cannot be empty".to_string(),
            ));
        }
        if provider.price_feed_id.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "price_feed_id cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            identity,
            token,
            provider,
        })
    }

    #[inline]
    pub fn id(&self) -> &str {
        &self.identity.asset_id
    }

    #[inline]
    pub fn symbol(&self) -> &str {
        &self.identity.symbol
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.identity.name
    }

    #[inline]
    pub fn mint(&self) -> &str {
        &self.token.mint
    }

    #[inline]
    pub fn decimals(&self) -> u8 {
        self.token.decimals
    }

    #[inline]
    pub fn price_feed_id(&self) -> &str {
        &self.provider.price_feed_id
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.provider.status == AssetStatus::Active
    }
}

/// Dynamic market data decoupled from static asset identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetMarketData {
    pub asset_id: String,
    pub symbol: String,
    /// Current USD price as f64
    pub price_usd: f64,
    /// Price in micro-USD (6 decimals, e.g. $125.50 -> 125_500_000)
    pub price_scaled: u64,
    /// Confidence interval in USD
    pub conf_usd: f64,
    /// Confidence width in basis points
    pub conf_bps: u16,
    /// Price exponent
    pub expo: i32,
    /// Publish Unix timestamp (seconds)
    pub publish_time: i64,
    /// Ingestion Unix timestamp (seconds)
    pub received_at: i64,
    /// Staleness flag
    pub is_stale: bool,
}

impl AssetMarketData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        asset_id: impl Into<String>,
        symbol: impl Into<String>,
        price_usd: f64,
        conf_usd: f64,
        expo: i32,
        publish_time: i64,
        received_at: i64,
        max_staleness_secs: i64,
    ) -> Result<Self, ValidationError> {
        if price_usd <= 0.0 {
            return Err(ValidationError::InvalidParam(format!(
                "Price must be positive, got {}",
                price_usd
            )));
        }

        let price_scaled = (price_usd * 1_000_000.0).round().max(0.0) as u64;
        let conf_bps = if price_usd > 0.0 {
            ((conf_usd / price_usd) * 10_000.0).round().min(10_000.0) as u16
        } else {
            10_000
        };

        let is_stale = (received_at - publish_time).abs() > max_staleness_secs;

        Ok(Self {
            asset_id: asset_id.into(),
            symbol: symbol.into(),
            price_usd,
            price_scaled,
            conf_usd,
            conf_bps,
            expo,
            publish_time,
            received_at,
            is_stale,
        })
    }
}

/// Canonical Collateral & Quote Token Mints on Solana.
pub const MAINNET_USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const DEVNET_USDC_MINT: &str = "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr";
pub const WRAPPED_SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Canonical Backed / Tokenized Equity Mints on Solana.
pub const BACKED_NVDA_MINT: &str = "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh";
pub const BACKED_AAPL_MINT: &str = "XsbEhLAtcf6HdfpFZ5xEMdqW8nfAvcsP5bdudRLJzJp";
pub const BACKED_SPYX_MINT: &str = "XsoCS1TfEyfFhfvj8EtZ528L3CaKBDBRqRapnBbDF2W";

/// Canonical Verified Meteora DBC Pools for Mainnet Equities.
pub const METEORA_NVDA_USDC_POOL: &str = "JCqWLp5RAaC3FPFX8MoAt7yRuPqRW2W9ZG3Byigxd1A7";
pub const METEORA_AAPL_USDC_POOL: &str = "C3Zm5CTFQxfCdbbDameKXRsmMUz8nfpkHqrevmdX97YS";
pub const METEORA_SPYX_USDC_POOL: &str = "CNutHtA6JUuwwWGCcJXobusHdRWZ4EgJTSUxxzXqnRj7";

/// Returns verified production assets with confirmed on-chain mints.
pub fn verified_mainnet_assets() -> Vec<Asset> {
    vec![
        Asset::new(
            AssetIdentity {
                asset_id: "backed:NVDAx".to_string(),
                symbol: "NVDA".to_string(),
                name: "Backed NVIDIA (NVDAx)".to_string(),
                asset_type: AssetType::Stock,
                underlying_reference: "NASDAQ:NVDA (ISIN US67066G1040)".to_string(),
            },
            TokenDetails {
                mint: BACKED_NVDA_MINT.to_string(),
                decimals: 8,
                network: Network::SolanaMainnet,
            },
            ProviderConfig {
                provider: AssetProvider::PreStocks,
                price_feed_id: "3155e714652285e6834d8ef0b3558163f4585c5b9679f222956cf57fb3645391"
                    .to_string(),
                meteora_pool: Some(METEORA_NVDA_USDC_POOL.to_string()),
                secondary_reference: Some("tessera:NVDA".to_string()),
                status: AssetStatus::Active,
            },
        )
        .expect("Valid NVDA asset"),
        Asset::new(
            AssetIdentity {
                asset_id: "backed:AAPLx".to_string(),
                symbol: "AAPL".to_string(),
                name: "Backed Apple (AAPLx)".to_string(),
                asset_type: AssetType::Stock,
                underlying_reference: "NASDAQ:AAPL (ISIN US0378331005)".to_string(),
            },
            TokenDetails {
                mint: BACKED_AAPL_MINT.to_string(),
                decimals: 8,
                network: Network::SolanaMainnet,
            },
            ProviderConfig {
                provider: AssetProvider::PreStocks,
                price_feed_id: "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175ec697df854ab"
                    .to_string(),
                meteora_pool: Some(METEORA_AAPL_USDC_POOL.to_string()),
                secondary_reference: Some("tessera:AAPL".to_string()),
                status: AssetStatus::Active,
            },
        )
        .expect("Valid AAPL asset"),
        Asset::new(
            AssetIdentity {
                asset_id: "backed:SPYx".to_string(),
                symbol: "SPYx".to_string(),
                name: "Backed S&P 500 Index (SPYx)".to_string(),
                asset_type: AssetType::Index,
                underlying_reference: "NYSEArca:SPY (Swiss DLT Act)".to_string(),
            },
            TokenDetails {
                mint: BACKED_SPYX_MINT.to_string(),
                decimals: 8,
                network: Network::SolanaMainnet,
            },
            ProviderConfig {
                provider: AssetProvider::PreStocks,
                price_feed_id: "2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b"
                    .to_string(),
                meteora_pool: Some(METEORA_SPYX_USDC_POOL.to_string()),
                secondary_reference: None,
                status: AssetStatus::Active,
            },
        )
        .expect("Valid SPYx asset"),
    ]
}

/// Canonical asset lifecycle and approval state machine.
///
/// Conceptual Progression:
/// PENDING -> VALIDATED -> SHARIAH_APPROVED -> ACTIVE
/// ACTIVE -> DEACTIVATED
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssetApprovalStatus {
    Pending,
    Validated,
    ShariahApproved,
    Active,
    Deactivated,
}

impl fmt::Display for AssetApprovalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Validated => write!(f, "VALIDATED"),
            Self::ShariahApproved => write!(f, "SHARIAH_APPROVED"),
            Self::Active => write!(f, "ACTIVE"),
            Self::Deactivated => write!(f, "DEACTIVATED"),
        }
    }
}

impl std::str::FromStr for AssetApprovalStatus {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "PENDING" => Ok(Self::Pending),
            "VALIDATED" => Ok(Self::Validated),
            "SHARIAH_APPROVED" => Ok(Self::ShariahApproved),
            "ACTIVE" => Ok(Self::Active),
            "DEACTIVATED" => Ok(Self::Deactivated),
            other => Err(ValidationError::InvalidParam(format!(
                "Invalid asset approval status: '{}'",
                other
            ))),
        }
    }
}

impl AssetApprovalStatus {
    /// Determines whether transition from current status to `target` is permitted.
    pub fn can_transition_to(&self, target: AssetApprovalStatus) -> bool {
        match (self, target) {
            // PENDING -> VALIDATED
            (Self::Pending, Self::Validated) => true,
            // VALIDATED -> SHARIAH_APPROVED
            (Self::Validated, Self::ShariahApproved) => true,
            // SHARIAH_APPROVED -> ACTIVE
            (Self::ShariahApproved, Self::Active) => true,
            // ACTIVE -> DEACTIVATED
            (Self::Active, Self::Deactivated) => true,
            // DEACTIVATED -> ACTIVE (re-activation) or DEACTIVATED -> PENDING (re-review)
            (Self::Deactivated, Self::Active) => true,
            (Self::Deactivated, Self::Pending) => true,
            // Idempotent self-transitions
            (a, b) if *a == b => true,
            // All other transitions prohibited
            _ => false,
        }
    }

    /// Whether this status indicates Shariah compliance approval.
    #[inline]
    pub fn is_shariah_approved(&self) -> bool {
        matches!(self, Self::ShariahApproved | Self::Active)
    }

    /// Whether this status represents an actively trading asset.
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// Canonical asset record rooted in mint identity, legally attested issuer,
/// and custodian backing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalAssetRecord {
    pub asset_id: String,
    pub symbol: String,
    pub mint_address: String,
    pub legal_issuer: String,
    pub custodian: String,
    pub underlying_asset_identifier: String,
    pub is_active: bool,
    pub approval_status: AssetApprovalStatus,
    pub decimals: u8,
    pub created_at: i64,
    pub updated_at: i64,
}

impl CanonicalAssetRecord {
    pub fn new(
        asset_id: impl Into<String>,
        symbol: impl Into<String>,
        mint_address: impl Into<String>,
        legal_issuer: impl Into<String>,
        custodian: impl Into<String>,
        underlying_asset_identifier: impl Into<String>,
        decimals: u8,
    ) -> Result<Self, ValidationError> {
        let asset_id = asset_id.into();
        let symbol = symbol.into();
        let mint_address = mint_address.into();
        let legal_issuer = legal_issuer.into();
        let custodian = custodian.into();
        let underlying_asset_identifier = underlying_asset_identifier.into();

        if asset_id.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "asset_id cannot be empty".to_string(),
            ));
        }
        if symbol.trim().is_empty() {
            return Err(ValidationError::EmptySymbol);
        }
        if mint_address.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "mint_address cannot be empty".to_string(),
            ));
        }
        if legal_issuer.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "legal_issuer cannot be empty".to_string(),
            ));
        }
        if custodian.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "custodian cannot be empty".to_string(),
            ));
        }
        if underlying_asset_identifier.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "underlying_asset_identifier cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            asset_id,
            symbol,
            mint_address,
            legal_issuer,
            custodian,
            underlying_asset_identifier,
            is_active: false,
            approval_status: AssetApprovalStatus::Pending,
            decimals,
            created_at: 0,
            updated_at: 0,
        })
    }

    /// Transitions lifecycle status if permitted by state machine rules.
    pub fn transition_to(&mut self, next: AssetApprovalStatus) -> Result<(), ValidationError> {
        if !self.approval_status.can_transition_to(next) {
            return Err(ValidationError::InvalidParam(format!(
                "Invalid status transition from {:?} to {:?}",
                self.approval_status, next
            )));
        }
        self.approval_status = next;
        self.is_active = next == AssetApprovalStatus::Active;
        Ok(())
    }

    /// Activates the asset for live market-data and trading.
    /// Fails closed if the asset has not been Shariah approved.
    pub fn activate(&mut self) -> Result<(), ValidationError> {
        if self.approval_status != AssetApprovalStatus::ShariahApproved
            && self.approval_status != AssetApprovalStatus::Active
        {
            return Err(ValidationError::InvalidParam(format!(
                "Cannot activate asset with status {:?}; must be SHARIAH_APPROVED first",
                self.approval_status
            )));
        }
        self.approval_status = AssetApprovalStatus::Active;
        self.is_active = true;
        Ok(())
    }

    /// Deactivates the asset, immediately removing it from live trading and market data feeds.
    pub fn deactivate(&mut self) {
        self.approval_status = AssetApprovalStatus::Deactivated;
        self.is_active = false;
    }
}

/// Explicit database-backed mapping between an approved canonical asset and its Pyth price feed.
///
/// Decouples market data feed resolution from ticker symbols, names, or conventions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetMarketDataMapping {
    pub mapping_id: Option<String>,
    pub asset_id: String,
    pub symbol: String,
    pub mint_address: String,
    pub pyth_feed_id: String,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl AssetMarketDataMapping {
    pub fn new(
        asset_id: impl Into<String>,
        symbol: impl Into<String>,
        mint_address: impl Into<String>,
        pyth_feed_id: impl Into<String>,
    ) -> Result<Self, ValidationError> {
        let asset_id = asset_id.into();
        let symbol = symbol.into();
        let mint_address = mint_address.into();
        let pyth_feed_id = pyth_feed_id.into().trim_start_matches("0x").to_lowercase();

        if asset_id.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "asset_id cannot be empty".to_string(),
            ));
        }
        if symbol.trim().is_empty() {
            return Err(ValidationError::EmptySymbol);
        }
        if mint_address.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "mint_address cannot be empty".to_string(),
            ));
        }
        if pyth_feed_id.trim().is_empty() {
            return Err(ValidationError::InvalidParam(
                "pyth_feed_id cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            mapping_id: None,
            asset_id,
            symbol,
            mint_address,
            pyth_feed_id,
            is_active: true,
            created_at: 0,
            updated_at: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_asset() -> Asset {
        Asset::new(
            AssetIdentity {
                asset_id: "prestocks:NVDA".to_string(),
                symbol: "NVDA".to_string(),
                name: "NVIDIA Corporation".to_string(),
                asset_type: AssetType::Stock,
                underlying_reference: "NASDAQ:NVDA".to_string(),
            },
            TokenDetails {
                mint: "NvdaMint111111111111111111111111111111111111".to_string(),
                decimals: 6,
                network: Network::SolanaDevnet,
            },
            ProviderConfig {
                provider: AssetProvider::PreStocks,
                price_feed_id: "3155e714652285e6834d8ef0b3558163f4585c5b9679f222956cf57fb3645391"
                    .to_string(),
                meteora_pool: Some("MeteoraNvdaPool1111111111111111111111111111".to_string()),
                secondary_reference: Some("tessera:NVDA".to_string()),
                status: AssetStatus::Active,
            },
        )
        .expect("Valid asset creation")
    }

    #[test]
    fn test_valid_asset_creation() {
        let asset = sample_asset();
        assert_eq!(asset.id(), "prestocks:NVDA");
        assert_eq!(asset.symbol(), "NVDA");
        assert_eq!(asset.name(), "NVIDIA Corporation");
        assert_eq!(asset.decimals(), 6);
        assert!(asset.is_active());
    }

    #[test]
    fn test_asset_validation_failures() {
        let mut identity = AssetIdentity {
            asset_id: "test:AAPL".to_string(),
            symbol: "".to_string(),
            name: "Apple".to_string(),
            asset_type: AssetType::Stock,
            underlying_reference: "NASDAQ:AAPL".to_string(),
        };
        let token = TokenDetails {
            mint: "Mint111111111111111111111111111111111111111".to_string(),
            decimals: 6,
            network: Network::SolanaDevnet,
        };
        let provider = ProviderConfig {
            provider: AssetProvider::PreStocks,
            price_feed_id: "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175ec697df854ab"
                .to_string(),
            meteora_pool: None,
            secondary_reference: None,
            status: AssetStatus::Active,
        };

        // Empty symbol
        assert_eq!(
            Asset::new(identity.clone(), token.clone(), provider.clone()).unwrap_err(),
            ValidationError::EmptySymbol
        );

        // Empty asset_id
        identity.symbol = "AAPL".to_string();
        identity.asset_id = "   ".to_string();
        assert!(Asset::new(identity.clone(), token.clone(), provider.clone()).is_err());

        // Empty mint
        identity.asset_id = "test:AAPL".to_string();
        let mut bad_token = token.clone();
        bad_token.mint = "".to_string();
        assert!(Asset::new(identity.clone(), bad_token, provider.clone()).is_err());

        // Empty price_feed_id
        let mut bad_provider = provider.clone();
        bad_provider.price_feed_id = "".to_string();
        assert!(Asset::new(identity, token, bad_provider).is_err());
    }

    #[test]
    fn test_asset_market_data_math_and_staleness() {
        let now = 1726000000;
        let data = AssetMarketData::new(
            "prestocks:NVDA",
            "NVDA",
            125.50,
            0.25, // conf
            -8,
            now,
            now + 30, // received 30s later
            60,       // max staleness 60s
        )
        .expect("Valid market data");

        assert_eq!(data.symbol, "NVDA");
        assert_eq!(data.price_scaled, 125_500_000);
        assert!(!data.is_stale);
        // conf_bps = (0.25 / 125.50) * 10000 = ~19.92 bps -> 20 bps
        assert_eq!(data.conf_bps, 20);

        // Negative price rejection
        assert!(AssetMarketData::new("id", "SYM", -10.0, 0.1, -8, now, now, 60).is_err());
        assert!(AssetMarketData::new("id", "SYM", 0.0, 0.1, -8, now, now, 60).is_err());

        let stale_data = AssetMarketData::new(
            "prestocks:NVDA",
            "NVDA",
            125.50,
            0.25,
            -8,
            now,
            now + 120, // 120s later vs 60s max
            60,
        )
        .expect("Valid parse but stale");
        assert!(stale_data.is_stale);
    }

    #[test]
    fn test_verify_supported_token_mints() {
        let assets = verified_mainnet_assets();
        assert_eq!(assets.len(), 3);

        for asset in &assets {
            assert!(!asset.mint().is_empty());
            assert_eq!(asset.decimals(), 8);
            assert!(asset.is_active());
            assert_eq!(asset.token.network, Network::SolanaMainnet);
            assert!(asset.provider.meteora_pool.is_some());
        }

        // Verify canonical constants
        assert_eq!(
            MAINNET_USDC_MINT,
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        );
        assert_eq!(
            DEVNET_USDC_MINT,
            "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr"
        );
        assert_eq!(
            WRAPPED_SOL_MINT,
            "So11111111111111111111111111111111111111112"
        );
        assert_eq!(
            BACKED_NVDA_MINT,
            "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh"
        );
        assert_eq!(
            BACKED_AAPL_MINT,
            "XsbEhLAtcf6HdfpFZ5xEMdqW8nfAvcsP5bdudRLJzJp"
        );
        assert_eq!(
            BACKED_SPYX_MINT,
            "XsoCS1TfEyfFhfvj8EtZ528L3CaKBDBRqRapnBbDF2W"
        );
        assert_eq!(
            METEORA_NVDA_USDC_POOL,
            "JCqWLp5RAaC3FPFX8MoAt7yRuPqRW2W9ZG3Byigxd1A7"
        );
        assert_eq!(
            METEORA_AAPL_USDC_POOL,
            "C3Zm5CTFQxfCdbbDameKXRsmMUz8nfpkHqrevmdX97YS"
        );
        assert_eq!(
            METEORA_SPYX_USDC_POOL,
            "CNutHtA6JUuwwWGCcJXobusHdRWZ4EgJTSUxxzXqnRj7"
        );
    }

    #[test]
    fn test_canonical_asset_lifecycle_transitions() {
        let mut asset = CanonicalAssetRecord::new(
            "backed:NVDAx",
            "NVDA",
            BACKED_NVDA_MINT,
            "Backed Finance AG",
            "Maerki Baumann & Co. AG",
            "NASDAQ:NVDA (ISIN US67066G1040)",
            8,
        )
        .expect("Valid canonical asset");

        assert_eq!(asset.approval_status, AssetApprovalStatus::Pending);
        assert!(!asset.is_active);
        assert!(!asset.approval_status.is_shariah_approved());

        // Cannot activate directly from PENDING
        assert!(asset.activate().is_err());

        // PENDING -> VALIDATED
        assert!(asset.transition_to(AssetApprovalStatus::Validated).is_ok());
        assert_eq!(asset.approval_status, AssetApprovalStatus::Validated);
        assert!(!asset.is_active);

        // Cannot activate directly from VALIDATED
        assert!(asset.activate().is_err());

        // VALIDATED -> SHARIAH_APPROVED
        assert!(asset
            .transition_to(AssetApprovalStatus::ShariahApproved)
            .is_ok());
        assert_eq!(asset.approval_status, AssetApprovalStatus::ShariahApproved);
        assert!(asset.approval_status.is_shariah_approved());
        assert!(!asset.is_active);

        // SHARIAH_APPROVED -> ACTIVE (Activation)
        assert!(asset.activate().is_ok());
        assert_eq!(asset.approval_status, AssetApprovalStatus::Active);
        assert!(asset.is_active);
        assert!(asset.approval_status.is_active());
        assert!(asset.approval_status.is_shariah_approved());

        // ACTIVE -> DEACTIVATED
        asset.deactivate();
        assert_eq!(asset.approval_status, AssetApprovalStatus::Deactivated);
        assert!(!asset.is_active);

        // Invalid direct transition from DEACTIVATED to VALIDATED
        assert!(asset.transition_to(AssetApprovalStatus::Validated).is_err());
    }

    #[test]
    fn test_canonical_asset_validation_failures() {
        // Empty asset_id
        assert!(
            CanonicalAssetRecord::new("", "NVDA", "mint1", "issuer", "cust", "ref", 6).is_err()
        );
        // Empty symbol
        assert!(
            CanonicalAssetRecord::new("id1", "  ", "mint1", "issuer", "cust", "ref", 6).is_err()
        );
        // Empty mint
        assert!(CanonicalAssetRecord::new("id1", "NVDA", "", "issuer", "cust", "ref", 6).is_err());
        // Empty issuer
        assert!(CanonicalAssetRecord::new("id1", "NVDA", "mint1", "", "cust", "ref", 6).is_err());
        // Empty custodian
        assert!(
            CanonicalAssetRecord::new("id1", "NVDA", "mint1", "issuer", " ", "ref", 6).is_err()
        );
        // Empty underlying ref
        assert!(
            CanonicalAssetRecord::new("id1", "NVDA", "mint1", "issuer", "cust", "", 6).is_err()
        );
    }

    #[test]
    fn test_asset_approval_status_formatting_and_parsing() {
        assert_eq!(AssetApprovalStatus::Pending.to_string(), "PENDING");
        assert_eq!(AssetApprovalStatus::Validated.to_string(), "VALIDATED");
        assert_eq!(
            AssetApprovalStatus::ShariahApproved.to_string(),
            "SHARIAH_APPROVED"
        );
        assert_eq!(AssetApprovalStatus::Active.to_string(), "ACTIVE");
        assert_eq!(AssetApprovalStatus::Deactivated.to_string(), "DEACTIVATED");

        assert_eq!(
            "pending".parse::<AssetApprovalStatus>().unwrap(),
            AssetApprovalStatus::Pending
        );
        assert_eq!(
            "SHARIAH_APPROVED".parse::<AssetApprovalStatus>().unwrap(),
            AssetApprovalStatus::ShariahApproved
        );
        assert_eq!(
            "active".parse::<AssetApprovalStatus>().unwrap(),
            AssetApprovalStatus::Active
        );
        assert_eq!(
            "deactivated".parse::<AssetApprovalStatus>().unwrap(),
            AssetApprovalStatus::Deactivated
        );
        assert!("unknown_status".parse::<AssetApprovalStatus>().is_err());
    }
}
