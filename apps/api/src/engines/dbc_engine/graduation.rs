use crate::engines::dbc_engine::config::GraduationTargetConfig;

/// Generates graduation parameters for migrating the pool into Meteora DAMM v2.
pub fn generate_graduation_config(
    graduation_threshold: f64,
    quote_decimals: u8,
) -> GraduationTargetConfig {
    let lamports = (graduation_threshold * 10_f64.powi(quote_decimals as i32)).round() as u64;

    GraduationTargetConfig {
        migration_option: "MET_DAMM_V2".to_string(),
        target_damm: "Meteora Dynamic AMM v2".to_string(),
        migration_quote_threshold: graduation_threshold,
        migration_quote_threshold_lamports: lamports,
        fee_option: "Customizable".to_string(),
        migrated_pool_fee_bps: 100, // 1.0% DAMM v2 pool fee
        partner_lp_percentage: 100, // 100% LP allocated to Equity Catalyst Vault
    }
}
