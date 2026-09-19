//! Deterministic Shariah-compliant Fee Model.
//!
//! Islamic commercial jurisprudence requires that all fees represent consideration
//! for genuine economic services (*Ujrah*), administrative facilitation, or asset
//! liquidity provision. Fees must be clearly defined, predictable, and free from
//! ambiguity (*Gharar*), interest (*Riba*), and punitive compounding penalties.
//!
//! This module provides deterministic fee calculation and validation:
//! - Pool Liquidity Fee (compensating LPs for inventory exposure, e.g. 15 bps)
//! - Platform Controller Fee (compensating protocol for on-chain risk evaluation and Shariah gatekeeping, e.g. 5 bps)
//! - Solana Network Fee (actual/estimated transaction execution cost in lamports)

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current canonical fee calculation policy version
pub const FEE_CALCULATION_VERSION: &str = "v1.0-deterministic";

/// Default pool liquidity provider fee in basis points (15 bps = 0.15%)
pub const DEFAULT_POOL_FEE_BPS: u16 = 15;

/// Default platform controller / Shariah verification fee in basis points (5 bps = 0.05%)
pub const DEFAULT_PLATFORM_FEE_BPS: u16 = 5;

/// Default estimated Solana transaction fee in lamports (5,000 lamports = 0.000005 SOL)
pub const DEFAULT_ESTIMATED_NETWORK_FEE_LAMPORTS: u64 = 5_000;

/// Default allowable network fee drift in lamports before flagging excessive gas inflation
pub const DEFAULT_MAX_NETWORK_FEE_DRIFT_LAMPORTS: u64 = 50_000;

/// Transparent fee breakdown disclosed to user before trade execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeBreakdown {
    /// Pool liquidity provider fee in raw token atomic units
    pub pool_fee: u64,
    /// Platform/protocol controller fee in raw token atomic units
    pub platform_fee: u64,
    /// Solana network execution fee in lamports (estimated or actual)
    pub network_fee: u64,
    /// Total token fees (pool_fee + platform_fee)
    pub total_fee: u64,
    /// Identifier of the deterministic fee formula version
    pub fee_calculation_version: String,

    // Optional human-readable / USD display values for frontend presentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_fee_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_fee_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_fee_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_fee_usd: Option<f64>,
}

/// Errors occurring during fee disclosure verification.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FeeValidationError {
    #[error("Required fee disclosure is missing from execution request")]
    MissingFeeDisclosure,

    #[error("Fee calculation version mismatch: expected '{expected}', got '{actual}'")]
    VersionMismatch { expected: String, actual: String },

    #[error("Pool fee mismatch: quoted {quoted} does not match expected {expected}")]
    PoolFeeMismatch { quoted: u64, expected: u64 },

    #[error("Platform fee mismatch: quoted {quoted} does not match expected {expected}")]
    PlatformFeeMismatch { quoted: u64, expected: u64 },

    #[error("Total fee mismatch: quoted {quoted} does not match expected {expected}")]
    TotalFeeMismatch { quoted: u64, expected: u64 },

    #[error(
        "Network fee drift exceeded threshold: quoted {quoted}, actual {actual}, max allowed variance {max_allowed}"
    )]
    ExcessiveNetworkFeeDrift {
        quoted: u64,
        actual: u64,
        max_allowed: u64,
    },
}

/// Deterministic fee schedule specifying rates in basis points and network fee baselines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeSchedule {
    pub pool_fee_bps: u16,
    pub platform_fee_bps: u16,
    pub estimated_network_fee_lamports: u64,
    pub version: String,
}

impl Default for FeeSchedule {
    fn default() -> Self {
        Self::standard_v1()
    }
}

impl FeeSchedule {
    /// Returns the standard v1.0 deterministic fee schedule (15 bps pool + 5 bps platform).
    pub fn standard_v1() -> Self {
        Self {
            pool_fee_bps: DEFAULT_POOL_FEE_BPS,
            platform_fee_bps: DEFAULT_PLATFORM_FEE_BPS,
            estimated_network_fee_lamports: DEFAULT_ESTIMATED_NETWORK_FEE_LAMPORTS,
            version: FEE_CALCULATION_VERSION.to_string(),
        }
    }

    /// Computes deterministic fee breakdown with half-up integer rounding.
    ///
    /// Mathematical properties:
    /// - Uses 128-bit intermediate products to prevent integer overflow.
    /// - Applies half-up rounding: `(amount * bps + 5_000) / 10_000`.
    /// - Guarantees `total_fee == pool_fee + platform_fee`.
    pub fn calculate_fees(
        &self,
        amount_in: u64,
        decimals: u8,
        sol_price_usd: Option<f64>,
    ) -> FeeBreakdown {
        let pool_fee = ((amount_in as u128 * self.pool_fee_bps as u128 + 5_000) / 10_000) as u64;
        let platform_fee =
            ((amount_in as u128 * self.platform_fee_bps as u128 + 5_000) / 10_000) as u64;
        let total_fee = pool_fee.saturating_add(platform_fee);
        let network_fee = self.estimated_network_fee_lamports;

        let scale = 10f64.powi(decimals as i32);
        let pool_fee_usd = (pool_fee as f64) / scale;
        let platform_fee_usd = (platform_fee as f64) / scale;
        let network_fee_usd =
            sol_price_usd.map(|price| (network_fee as f64 / 1_000_000_000.0) * price);
        let total_fee_usd = pool_fee_usd + platform_fee_usd + network_fee_usd.unwrap_or(0.0);

        FeeBreakdown {
            pool_fee,
            platform_fee,
            network_fee,
            total_fee,
            fee_calculation_version: self.version.clone(),
            pool_fee_usd: Some(pool_fee_usd),
            platform_fee_usd: Some(platform_fee_usd),
            network_fee_usd,
            total_fee_usd: Some(total_fee_usd),
        }
    }

    /// Validates an incoming trade's fee disclosure against the deterministic schedule.
    ///
    /// Ensures that the execution result CANNOT silently differ from the quoted fees:
    /// - Calculation version must match.
    /// - Pool fee must match exactly.
    /// - Platform fee must match exactly.
    /// - Total fee must equal pool_fee + platform_fee.
    /// - Network fee drift must remain within clearly defined bounds.
    pub fn validate_fee_disclosure(
        &self,
        quoted: &FeeBreakdown,
        amount_in: u64,
        max_network_fee_drift_lamports: Option<u64>,
    ) -> Result<(), FeeValidationError> {
        if quoted.fee_calculation_version != self.version {
            return Err(FeeValidationError::VersionMismatch {
                expected: self.version.clone(),
                actual: quoted.fee_calculation_version.clone(),
            });
        }

        let expected_pool_fee =
            ((amount_in as u128 * self.pool_fee_bps as u128 + 5_000) / 10_000) as u64;
        if quoted.pool_fee != expected_pool_fee {
            return Err(FeeValidationError::PoolFeeMismatch {
                quoted: quoted.pool_fee,
                expected: expected_pool_fee,
            });
        }

        let expected_platform_fee =
            ((amount_in as u128 * self.platform_fee_bps as u128 + 5_000) / 10_000) as u64;
        if quoted.platform_fee != expected_platform_fee {
            return Err(FeeValidationError::PlatformFeeMismatch {
                quoted: quoted.platform_fee,
                expected: expected_platform_fee,
            });
        }

        let expected_total_fee = expected_pool_fee.saturating_add(expected_platform_fee);
        if quoted.total_fee != expected_total_fee {
            return Err(FeeValidationError::TotalFeeMismatch {
                quoted: quoted.total_fee,
                expected: expected_total_fee,
            });
        }

        // Bounded network fee drift check
        let max_drift =
            max_network_fee_drift_lamports.unwrap_or(DEFAULT_MAX_NETWORK_FEE_DRIFT_LAMPORTS);
        let drift =
            (quoted.network_fee as i64 - self.estimated_network_fee_lamports as i64).unsigned_abs();
        if drift > max_drift {
            return Err(FeeValidationError::ExcessiveNetworkFeeDrift {
                quoted: quoted.network_fee,
                actual: self.estimated_network_fee_lamports,
                max_allowed: max_drift,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_five_hundred_dollar_trade_exact_calculations() {
        let schedule = FeeSchedule::standard_v1();
        // $500.00 in USDC (6 decimals) = 500_000_000 atomic units
        let amount_in = 500_000_000u64;
        let breakdown = schedule.calculate_fees(amount_in, 6, Some(150.0));

        // Pool fee: 15 bps = 0.15% of $500.00 = $0.75 (750,000 atomic units)
        assert_eq!(breakdown.pool_fee, 750_000);
        assert_eq!(breakdown.pool_fee_usd, Some(0.75));

        // Platform fee: 5 bps = 0.05% of $500.00 = $0.25 (250,000 atomic units)
        assert_eq!(breakdown.platform_fee, 250_000);
        assert_eq!(breakdown.platform_fee_usd, Some(0.25));

        // Total trading fee: $1.00 (1,000_000 atomic units)
        assert_eq!(breakdown.total_fee, 1_000_000);

        // Network fee: 5,000 lamports
        assert_eq!(breakdown.network_fee, 5_000);
        assert_eq!(
            breakdown.network_fee_usd,
            Some((5_000f64 / 1_000_000_000.0) * 150.0)
        );

        // Version matches
        assert_eq!(breakdown.fee_calculation_version, "v1.0-deterministic");

        // Validation passes
        assert!(schedule
            .validate_fee_disclosure(&breakdown, amount_in, None)
            .is_ok());
    }

    #[test]
    fn test_rounding_boundaries_half_up() {
        let schedule = FeeSchedule::standard_v1();

        // Test amount where remainder is exactly 0.5 (half-up)
        // For 15 bps: (amount * 15 + 5000) / 10000
        // When amount = 333: 333 * 15 = 4995 -> (4995 + 5000) / 10000 = 0
        // When amount = 334: 334 * 15 = 5010 -> (5010 + 5000) / 10000 = 1
        let breakdown_333 = schedule.calculate_fees(333, 6, None);
        assert_eq!(breakdown_333.pool_fee, 0);

        let breakdown_334 = schedule.calculate_fees(334, 6, None);
        assert_eq!(breakdown_334.pool_fee, 1);

        // For platform fee (5 bps):
        // When amount = 999: 999 * 5 = 4995 -> 0
        // When amount = 1000: 1000 * 5 = 5000 -> (5000 + 5000) / 10000 = 1
        let breakdown_999 = schedule.calculate_fees(999, 6, None);
        assert_eq!(breakdown_999.platform_fee, 0);

        let breakdown_1000 = schedule.calculate_fees(1000, 6, None);
        assert_eq!(breakdown_1000.platform_fee, 1);
    }

    #[test]
    fn test_validation_rejects_pool_fee_mismatch() {
        let schedule = FeeSchedule::standard_v1();
        let amount_in = 500_000_000u64;
        let mut breakdown = schedule.calculate_fees(amount_in, 6, None);

        // Tamper with pool fee (e.g. silent 1 unit drift)
        breakdown.pool_fee += 1;

        let err = schedule
            .validate_fee_disclosure(&breakdown, amount_in, None)
            .unwrap_err();
        assert_eq!(
            err,
            FeeValidationError::PoolFeeMismatch {
                quoted: 750_001,
                expected: 750_000,
            }
        );
    }

    #[test]
    fn test_validation_rejects_platform_fee_mismatch() {
        let schedule = FeeSchedule::standard_v1();
        let amount_in = 500_000_000u64;
        let mut breakdown = schedule.calculate_fees(amount_in, 6, None);

        // Tamper with platform fee
        breakdown.platform_fee -= 1;

        let err = schedule
            .validate_fee_disclosure(&breakdown, amount_in, None)
            .unwrap_err();
        assert_eq!(
            err,
            FeeValidationError::PlatformFeeMismatch {
                quoted: 249_999,
                expected: 250_000,
            }
        );
    }

    #[test]
    fn test_validation_rejects_version_mismatch() {
        let schedule = FeeSchedule::standard_v1();
        let amount_in = 500_000_000u64;
        let mut breakdown = schedule.calculate_fees(amount_in, 6, None);

        breakdown.fee_calculation_version = "v0.9-legacy".to_string();

        let err = schedule
            .validate_fee_disclosure(&breakdown, amount_in, None)
            .unwrap_err();
        assert_eq!(
            err,
            FeeValidationError::VersionMismatch {
                expected: "v1.0-deterministic".to_string(),
                actual: "v0.9-legacy".to_string(),
            }
        );
    }

    #[test]
    fn test_validation_rejects_excessive_network_fee_drift() {
        let schedule = FeeSchedule::standard_v1();
        let amount_in = 500_000_000u64;
        let mut breakdown = schedule.calculate_fees(amount_in, 6, None);

        // Inflated network fee (e.g. 500,000 lamports vs expected 5,000)
        breakdown.network_fee = 500_000;

        let err = schedule
            .validate_fee_disclosure(&breakdown, amount_in, Some(10_000))
            .unwrap_err();
        assert_eq!(
            err,
            FeeValidationError::ExcessiveNetworkFeeDrift {
                quoted: 500_000,
                actual: 5_000,
                max_allowed: 10_000,
            }
        );
    }
}
