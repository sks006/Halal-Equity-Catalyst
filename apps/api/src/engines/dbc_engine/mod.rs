pub mod adapter;
pub mod config;
pub mod curve;
pub mod graduation;
pub mod pricing;
pub mod simulator;
pub mod validation;

pub use adapter::{DbcError, LiquidityProvider, MeteoraDbcProvider};
pub use config::{
    CurveSegmentConfig, DbcConfigRequest, DbcConfigResponse, FeeStructureConfig,
    GraduationTargetConfig,
};
pub use simulator::{
    ComparisonSimulationRequest, ComparisonSimulationResponse, DbcSimulationInput,
    DbcSimulationResult, DbcSimulator, SimulationPricePoint, SimulationSegment,
};
pub use validation::DbcValidationError;

/// Meteora DBC Program ID on Devnet and Mainnet.
pub const METEORA_DBC_PROGRAM_ID: &str = "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN";

/// Core DBC Engine responsible for computing and validating Dynamic Bonding Curve configurations.
#[derive(Debug, Default, Clone)]
pub struct DbcEngine;

impl DbcEngine {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates and compiles a DBC configuration request into a validated DBC parameter specification.
    pub fn configure(
        &self,
        req: DbcConfigRequest,
    ) -> Result<DbcConfigResponse, DbcValidationError> {
        // 1. Validate inputs
        validation::validate_dbc_request(&req)?;

        let (quote_decimals, quote_mint) = validation::resolve_quote_token_info(&req.quote_token);
        let base_decimals: u8 = 6; // Standard 6 decimals for tokenized equity assets
        let total_supply = req.total_supply.unwrap_or(1_000_000.0);

        // 2. Generate curve segments for the specified profile (default: Equity Discovery 3-Regime)
        let segments = curve::generate_curve_segments(
            &req.curve_profile,
            req.initial_price,
            total_supply,
            base_decimals,
            quote_decimals,
        );

        // 3. Configure fee structure (Anti-sniping linear scheduler + dynamic volatility fee)
        let fee_structure = FeeStructureConfig {
            base_fee_mode: "FeeSchedulerLinear".to_string(),
            starting_fee_bps: 250,
            ending_fee_bps: 50,
            dynamic_volatility_fee_enabled: true,
            collect_fee_mode: "QuoteToken".to_string(),
            creator_fee_share_pct: 20,
            partner_fee_share_pct: 80,
        };

        // 4. Configure graduation to Meteora DAMM v2
        let graduation =
            graduation::generate_graduation_config(req.graduation_threshold, quote_decimals);

        let summary = format!(
            "Equity Discovery Curve compiled for {} pegged at ${:.2} {}. {} piecewise regimes (weights: 1x -> 4x -> 8x) targeting DAMM v2 migration at ${:.2} liquidity.",
            req.asset, req.initial_price, req.quote_token, segments.len(), req.graduation_threshold
        );

        Ok(DbcConfigResponse {
            asset: req.asset,
            quote_token: req.quote_token,
            quote_mint: quote_mint.to_string(),
            initial_price: req.initial_price,
            curve_profile: req.curve_profile,
            total_token_supply: total_supply,
            segments,
            fee_structure,
            graduation,
            program_id: METEORA_DBC_PROGRAM_ID.to_string(),
            summary,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_dbc_configuration() {
        let engine = DbcEngine::new();
        let req = DbcConfigRequest {
            asset: "TOKENIZED_STOCK".to_string(),
            quote_token: "USDC".to_string(),
            initial_price: 100.0,
            curve_profile: "equity_discovery".to_string(),
            graduation_threshold: 750.0,
            total_supply: Some(1_000_000.0),
        };

        let res = engine.configure(req).expect("Configuration must succeed");

        assert_eq!(res.asset, "TOKENIZED_STOCK");
        assert_eq!(res.quote_token, "USDC");
        assert_eq!(res.initial_price, 100.0);
        assert_eq!(res.segments.len(), 3);
        assert_eq!(res.segments[0].liquidity_weight, 1);
        assert_eq!(res.segments[1].liquidity_weight, 4);
        assert_eq!(res.segments[2].liquidity_weight, 8);
        assert_eq!(res.graduation.migration_option, "MET_DAMM_V2");
        assert_eq!(res.graduation.migration_quote_threshold, 750.0);
    }

    #[test]
    fn test_invalid_dbc_configuration_rejections() {
        let engine = DbcEngine::new();

        // Empty asset
        let req_empty_asset = DbcConfigRequest {
            asset: "".to_string(),
            quote_token: "USDC".to_string(),
            initial_price: 100.0,
            curve_profile: "equity_discovery".to_string(),
            graduation_threshold: 750.0,
            total_supply: None,
        };
        assert!(engine.configure(req_empty_asset).is_err());

        // Negative price
        let req_neg_price = DbcConfigRequest {
            asset: "TOKENIZED_STOCK".to_string(),
            quote_token: "USDC".to_string(),
            initial_price: -10.0,
            curve_profile: "equity_discovery".to_string(),
            graduation_threshold: 750.0,
            total_supply: None,
        };
        assert!(engine.configure(req_neg_price).is_err());
    }
}
