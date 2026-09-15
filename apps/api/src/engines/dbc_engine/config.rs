use serde::{Deserialize, Serialize};

/// Request payload to configure a Meteora Dynamic Bonding Curve.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DbcConfigRequest {
    pub asset: String,
    pub quote_token: String,
    pub initial_price: f64,
    pub curve_profile: String,
    pub graduation_threshold: f64,
    #[serde(default = "default_total_supply")]
    pub total_supply: Option<f64>,
}

fn default_total_supply() -> Option<f64> {
    Some(1_000_000.0)
}

/// Represents a validated piecewise curve segment in the DBC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurveSegmentConfig {
    pub segment_index: usize,
    pub regime_name: String,
    pub start_price: f64,
    pub end_price: f64,
    pub sqrt_price_start: String,
    pub sqrt_price_end: String,
    pub liquidity_weight: u32,
    pub estimated_liquidity: String,
    pub description: String,
}

/// Fee configuration including anti-sniping scheduler and dynamic volatility protection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeeStructureConfig {
    pub base_fee_mode: String,
    pub starting_fee_bps: u32,
    pub ending_fee_bps: u32,
    pub dynamic_volatility_fee_enabled: bool,
    pub collect_fee_mode: String,
    pub creator_fee_share_pct: u8,
    pub partner_fee_share_pct: u8,
}

/// Migration target to Meteora DAMM v2 upon hitting the graduation threshold.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraduationTargetConfig {
    pub migration_option: String,
    pub target_damm: String,
    pub migration_quote_threshold: f64,
    pub migration_quote_threshold_lamports: u64,
    pub fee_option: String,
    pub migrated_pool_fee_bps: u32,
    pub partner_lp_percentage: u8,
}

/// Fully validated and compiled DBC configuration ready for transaction construction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DbcConfigResponse {
    pub asset: String,
    pub quote_token: String,
    pub quote_mint: String,
    pub initial_price: f64,
    pub curve_profile: String,
    pub total_token_supply: f64,
    pub segments: Vec<CurveSegmentConfig>,
    pub fee_structure: FeeStructureConfig,
    pub graduation: GraduationTargetConfig,
    pub program_id: String,
    pub summary: String,
}
