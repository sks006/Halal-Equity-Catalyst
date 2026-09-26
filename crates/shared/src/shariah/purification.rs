//! Dedicated Shariah Dividend and Incidental Income Purification Module (Phase 15).
//!
//! # Core Islamic Finance Principle: Separation of Eligibility from Purification
//! In Islamic jurisprudence (AAOIFI Financial Papers Standard No. 21, §3/4; FTSE Shariah;
//! S&P Shariah Indices; and International Islamic Fiqh Academy resolutions):
//!
//! 1. **Eligibility Screening (Tamhiz)**:
//!    Evaluates whether an equity security is fundamentally permissible for spot investment based
//!    on qualitative business activities and financial debt/cash/receivables ratios. If incidental
//!    non-permissible income is below the policy threshold (e.g. `impure_income_ratio_bps <= 500 bps`
//!    / `5.00%`), the asset is certified as [`ShariahStatus::Approved`](super::types::ShariahStatus::Approved).
//!
//! 2. **Purification (Tathir al-Mal)**:
//!    Even though the company is Approved and eligible for spot investment, any incidental non-permissible
//!    earnings (e.g. interest earned on corporate operating cash deposits) distributed to token holders via
//!    dividends or cash distributions must be cleansed by donating the non-permissible portion to charitable
//!    causes (*Sadaqah li-tathir al-mal*) without claiming tax deductions or expecting spiritual reward.
//!
//! # Strict Invariant: Ratio != Payment Value
//! - `impure_income_ratio_bps` is a dimensionless fractional ratio in basis points (1 bp = 0.01% = 0.0001).
//! - `impure_income_value_minor_units` is an absolute monetary currency amount in minor units (e.g. cents, lamports).
//! - **Under no circumstances is the eligibility ratio itself used as the payment value.**
//!   For instance, an asset with a 120 bps (1.20%) impure income ratio distributing a $10,000.00 dividend
//!   (1,000,000 cents) requires a purification payment of **12,000 cents ($120.00)**, NOT 120 cents.
//!
//! # Explicit Units & Currency
//! All monetary amounts are denominated in explicit minor currency units with a paired [`CurrencyCode`].
//! Ambiguous fields like `amount`, `value`, or `ratio` without unit information are strictly avoided.
//!
//! # Phase 15 Scope Notice
//! In Phase 15, purification amounts are strictly computed, assessed, rounded, and reported.
//! **No on-chain payment execution or fund transfer is performed in this phase.**

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

use super::registry::RegisteredAsset;

/// Explicit currency code specifying the denomination of all minor-unit monetary values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CurrencyCode {
    #[default]
    Usdc,
    Usd,
    Eur,
    Gbp,
    Sol,
    Custom(String),
}

impl CurrencyCode {
    /// Returns the standard number of decimal places for minor unit representation.
    pub fn decimals(&self) -> u8 {
        match self {
            Self::Usdc => 6, // 1 USDC = 1,000,000 micro-units
            Self::Usd => 2,  // 1 USD = 100 cents
            Self::Eur => 2,  // 1 EUR = 100 cents
            Self::Gbp => 2,  // 1 GBP = 100 pence
            Self::Sol => 9,  // 1 SOL = 1,000,000,000 lamports
            Self::Custom(_) => 2,
        }
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usdc => write!(f, "USDC"),
            Self::Usd => write!(f, "USD"),
            Self::Eur => write!(f, "EUR"),
            Self::Gbp => write!(f, "GBP"),
            Self::Sol => write!(f, "SOL"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Deterministic rounding rule applied when calculating fractional minor currency units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PurificationRoundingRule {
    /// Conservative ceiling rounding (always rounds up fractional minor units).
    /// Standard in Islamic jurisprudence (*Ihtiyat* / religious precaution) to guarantee that
    /// no impermissible fractional wealth is retained by the investor or vault.
    #[default]
    ConservativeCeiling,
    /// Standard financial half-up rounding (>= 0.5 rounds up, < 0.5 rounds down).
    NearestHalfUp,
    /// Floor truncation (discards any fractional minor unit remainder).
    Floor,
}

impl fmt::Display for PurificationRoundingRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConservativeCeiling => write!(f, "ConservativeCeiling"),
            Self::NearestHalfUp => write!(f, "NearestHalfUp"),
            Self::Floor => write!(f, "Floor"),
        }
    }
}

/// Error encountered during purification calculation or validation.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum PurificationError {
    #[error("Purification ratio {impure_income_ratio_bps} bps exceeds maximum allowable 10,000 bps (100.00%)")]
    RatioExceedsMaximum { impure_income_ratio_bps: u32 },

    #[error("Direct impure income value ({impure_income_value_minor_units} minor units) exceeds gross dividend ({gross_dividend_value_minor_units} minor units)")]
    DirectValueExceedsGross {
        impure_income_value_minor_units: u64,
        gross_dividend_value_minor_units: u64,
    },

    #[error("Arithmetic overflow during purification calculation")]
    ArithmeticOverflow,

    #[error("Asset '{0}' is not approved for Shariah investment")]
    AssetNotApproved(String),
}

/// Input request for calculating dividend purification from a gross monetary dividend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DividendPurificationRequest {
    /// Canonical asset identifier (e.g. "backed:NVDA").
    pub asset_id: String,
    /// Explicit currency denomination for the dividend.
    pub currency_code: CurrencyCode,
    /// Total gross dividend received in minor currency units (e.g. cents).
    pub gross_dividend_value_minor_units: u64,
    /// Incidental impure income ratio in basis points (e.g. 120 bps = 1.20%).
    pub impure_income_ratio_bps: u32,
    /// Rounding methodology applied to fractional minor units.
    pub rounding_rule: PurificationRoundingRule,
}

/// Input request for calculating dividend purification on a per-share basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerShareDividendPurificationRequest {
    /// Canonical asset identifier.
    pub asset_id: String,
    /// Explicit currency denomination.
    pub currency_code: CurrencyCode,
    /// Total number of whole shares held.
    pub total_shares_count: u64,
    /// Dividend declared per single share, in minor currency units.
    pub dividend_per_share_value_minor_units: u64,
    /// Incidental impure income ratio in basis points.
    pub impure_income_ratio_bps: u32,
    /// Rounding methodology applied to fractional minor units.
    pub rounding_rule: PurificationRoundingRule,
}

/// Input request when impure income is directly reported in minor currency units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectValuePurificationRequest {
    /// Canonical asset identifier.
    pub asset_id: String,
    /// Explicit currency denomination.
    pub currency_code: CurrencyCode,
    /// Total gross dividend received in minor currency units.
    pub gross_dividend_value_minor_units: u64,
    /// Explicit monetary value of non-permissible income in minor currency units.
    pub impure_income_value_minor_units: u64,
}

/// Fully computed, deterministic purification assessment for a dividend or cash distribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurificationAssessment {
    /// Canonical asset identifier.
    #[serde(default)]
    pub asset_id: String,

    /// Explicit currency denomination of all minor-unit values.
    #[serde(default)]
    pub currency_code: CurrencyCode,

    /// Total gross dividend received in minor currency units.
    #[serde(alias = "dividend_amount_minor")]
    pub gross_dividend_value_minor_units: u64,

    /// Incidental impure income ratio in basis points (1 bp = 0.01%).
    #[serde(alias = "purification_ratio_bps")]
    pub impure_income_ratio_bps: u32,

    /// Calculated monetary value in minor currency units that must be donated to charity.
    /// Invariant: NEVER equal to the dimensionless ratio itself (unless by mathematical coincidence).
    #[serde(alias = "purification_amount_minor")]
    pub purification_value_minor_units: u64,

    /// Net permissible dividend value in minor currency units retained by the investor or vault.
    #[serde(alias = "net_permissible_amount_minor")]
    pub net_permissible_value_minor_units: u64,

    /// Rounding rule applied during calculation.
    #[serde(default)]
    pub rounding_rule: PurificationRoundingRule,

    /// True if non-permissible income is present and purification donation is required.
    #[serde(alias = "applicable")]
    pub purification_required: bool,
}

impl PurificationAssessment {
    /// Returns true if the dividend is completely pure (zero purification required).
    #[inline]
    pub fn is_pure(&self) -> bool {
        !self.purification_required || self.purification_value_minor_units == 0
    }

    /// Explicit accessor for gross dividend value in minor currency units.
    #[inline]
    pub fn gross_dividend_value_minor_units(&self) -> u64 {
        self.gross_dividend_value_minor_units
    }

    /// Explicit accessor for calculated purification value in minor currency units.
    #[inline]
    pub fn purification_value_minor_units(&self) -> u64 {
        self.purification_value_minor_units
    }

    /// Explicit accessor for net permissible dividend value in minor currency units.
    #[inline]
    pub fn net_permissible_value_minor_units(&self) -> u64 {
        self.net_permissible_value_minor_units
    }

    /// Explicit accessor for impure income ratio in basis points.
    #[inline]
    pub fn impure_income_ratio_bps(&self) -> u32 {
        self.impure_income_ratio_bps
    }

    /// Explicit accessor for the currency code.
    #[inline]
    pub fn currency_code(&self) -> &CurrencyCode {
        &self.currency_code
    }

    /// Explicit accessor for the rounding rule applied.
    #[inline]
    pub fn rounding_rule(&self) -> PurificationRoundingRule {
        self.rounding_rule
    }

    // --- Backwards-compatible accessors for legacy code ---

    /// Legacy accessor for gross dividend amount in minor units.
    #[inline]
    pub fn dividend_amount_minor(&self) -> u64 {
        self.gross_dividend_value_minor_units
    }

    /// Legacy accessor for calculated purification amount in minor units.
    #[inline]
    pub fn purification_amount_minor(&self) -> u64 {
        self.purification_value_minor_units
    }

    /// Legacy accessor for net permissible dividend amount in minor units.
    #[inline]
    pub fn net_permissible_amount_minor(&self) -> u64 {
        self.net_permissible_value_minor_units
    }

    /// Legacy accessor for purification ratio in basis points.
    #[inline]
    pub fn purification_ratio_bps(&self) -> u32 {
        self.impure_income_ratio_bps
    }

    /// Legacy accessor for applicable flag.
    #[inline]
    pub fn applicable(&self) -> bool {
        self.purification_required
    }
}

/// Deterministically computes the purification value in minor currency units from a gross
/// dividend value and an impure income ratio in basis points, applying explicit rounding rules.
///
/// # Mathematical Definition
/// $$\text{purification\_value\_minor\_units} = \text{round}\left(\frac{\text{gross\_dividend\_value\_minor\_units} \times \text{impure\_income\_ratio\_bps}}{10,000}\right)$$
///
/// # Invariants
/// 1. `impure_income_ratio_bps <= 10,000` (100.00%).
/// 2. If gross dividend is 0 or ratio is 0, the purification value is strictly 0.
/// 3. The purification value will never exceed `gross_dividend_value_minor_units`.
/// 4. All math is performed using checked 128-bit integer arithmetic (zero floating-point drift).
pub fn calculate_purification_value_minor_units(
    gross_dividend_value_minor_units: u64,
    impure_income_ratio_bps: u32,
    rounding_rule: PurificationRoundingRule,
) -> Result<u64, PurificationError> {
    if impure_income_ratio_bps > 10_000 {
        return Err(PurificationError::RatioExceedsMaximum {
            impure_income_ratio_bps,
        });
    }

    if gross_dividend_value_minor_units == 0 || impure_income_ratio_bps == 0 {
        return Ok(0);
    }

    let numerator = (gross_dividend_value_minor_units as u128)
        .checked_mul(impure_income_ratio_bps as u128)
        .ok_or(PurificationError::ArithmeticOverflow)?;

    let quotient = numerator / 10_000;
    let remainder = numerator % 10_000;

    let rounded_quotient = match rounding_rule {
        PurificationRoundingRule::ConservativeCeiling => {
            if remainder > 0 {
                quotient
                    .checked_add(1)
                    .ok_or(PurificationError::ArithmeticOverflow)?
            } else {
                quotient
            }
        }
        PurificationRoundingRule::NearestHalfUp => {
            if remainder * 2 >= 10_000 {
                quotient
                    .checked_add(1)
                    .ok_or(PurificationError::ArithmeticOverflow)?
            } else {
                quotient
            }
        }
        PurificationRoundingRule::Floor => quotient,
    };

    // Cap at gross dividend to guarantee never exceeding total distribution
    let capped = rounded_quotient.min(gross_dividend_value_minor_units as u128);

    if capped > u64::MAX as u128 {
        return Err(PurificationError::ArithmeticOverflow);
    }

    Ok(capped as u64)
}

/// Evaluates a [`DividendPurificationRequest`] and returns a complete [`PurificationAssessment`].
pub fn assess_dividend_purification(
    request: &DividendPurificationRequest,
) -> Result<PurificationAssessment, PurificationError> {
    let purification_value_minor_units = calculate_purification_value_minor_units(
        request.gross_dividend_value_minor_units,
        request.impure_income_ratio_bps,
        request.rounding_rule,
    )?;

    let net_permissible_value_minor_units = request
        .gross_dividend_value_minor_units
        .saturating_sub(purification_value_minor_units);

    Ok(PurificationAssessment {
        asset_id: request.asset_id.clone(),
        currency_code: request.currency_code.clone(),
        gross_dividend_value_minor_units: request.gross_dividend_value_minor_units,
        impure_income_ratio_bps: request.impure_income_ratio_bps,
        purification_value_minor_units,
        net_permissible_value_minor_units,
        rounding_rule: request.rounding_rule,
        purification_required: request.impure_income_ratio_bps > 0
            && request.gross_dividend_value_minor_units > 0
            && purification_value_minor_units > 0,
    })
}

/// Evaluates a [`PerShareDividendPurificationRequest`] by multiplying share count by per-share dividend
/// and applying the impure income ratio in basis points.
pub fn assess_per_share_dividend_purification(
    request: &PerShareDividendPurificationRequest,
) -> Result<PurificationAssessment, PurificationError> {
    let total_gross_dividend_u128 = (request.total_shares_count as u128)
        .checked_mul(request.dividend_per_share_value_minor_units as u128)
        .ok_or(PurificationError::ArithmeticOverflow)?;

    if total_gross_dividend_u128 > u64::MAX as u128 {
        return Err(PurificationError::ArithmeticOverflow);
    }

    let gross_dividend_value_minor_units = total_gross_dividend_u128 as u64;

    assess_dividend_purification(&DividendPurificationRequest {
        asset_id: request.asset_id.clone(),
        currency_code: request.currency_code.clone(),
        gross_dividend_value_minor_units,
        impure_income_ratio_bps: request.impure_income_ratio_bps,
        rounding_rule: request.rounding_rule,
    })
}

/// Evaluates a [`DirectValuePurificationRequest`] where non-permissible income is explicitly
/// provided in minor currency units (e.g. from custodian audit report or company breakdown).
pub fn assess_direct_impure_value(
    request: &DirectValuePurificationRequest,
) -> Result<PurificationAssessment, PurificationError> {
    if request.impure_income_value_minor_units > request.gross_dividend_value_minor_units {
        return Err(PurificationError::DirectValueExceedsGross {
            impure_income_value_minor_units: request.impure_income_value_minor_units,
            gross_dividend_value_minor_units: request.gross_dividend_value_minor_units,
        });
    }

    let net_permissible_value_minor_units = request
        .gross_dividend_value_minor_units
        .saturating_sub(request.impure_income_value_minor_units);

    // Compute effective basis points ratio for audit reporting
    let effective_ratio_bps = if request.gross_dividend_value_minor_units > 0 {
        let bps = ((request.impure_income_value_minor_units as u128 * 10_000)
            / request.gross_dividend_value_minor_units as u128) as u32;
        bps.min(10_000)
    } else {
        0
    };

    Ok(PurificationAssessment {
        asset_id: request.asset_id.clone(),
        currency_code: request.currency_code.clone(),
        gross_dividend_value_minor_units: request.gross_dividend_value_minor_units,
        impure_income_ratio_bps: effective_ratio_bps,
        purification_value_minor_units: request.impure_income_value_minor_units,
        net_permissible_value_minor_units,
        rounding_rule: PurificationRoundingRule::Floor,
        purification_required: request.impure_income_value_minor_units > 0,
    })
}

/// Evaluates purification for an admitted [`RegisteredAsset`] using its verified eligibility criteria.
///
/// Demonstrates strict independence between the Shariah eligibility status (`Approved`)
/// and the calculation of post-distribution charitable purification.
pub fn assess_purification_for_registered_asset(
    registered: &RegisteredAsset,
    gross_dividend_value_minor_units: u64,
    currency_code: CurrencyCode,
    rounding_rule: PurificationRoundingRule,
) -> Result<PurificationAssessment, PurificationError> {
    let impure_income_ratio_bps = registered.eligibility.impure_income_bps;

    assess_dividend_purification(&DividendPurificationRequest {
        asset_id: registered.asset.id().to_string(),
        currency_code,
        gross_dividend_value_minor_units,
        impure_income_ratio_bps,
        rounding_rule,
    })
}

// --- Backwards compatibility functions ---

/// Legacy calculation function computing purification amount using default floor rounding.
pub fn calculate_purification(
    dividend_amount_minor: u64,
    purification_ratio_bps: u32,
) -> Result<u64, PurificationError> {
    calculate_purification_value_minor_units(
        dividend_amount_minor,
        purification_ratio_bps,
        PurificationRoundingRule::Floor,
    )
}

/// Legacy assessment function generating a [`PurificationAssessment`] with default USD currency.
pub fn assess_purification(
    dividend_amount_minor: u64,
    purification_ratio_bps: u32,
) -> Result<PurificationAssessment, PurificationError> {
    assess_dividend_purification(&DividendPurificationRequest {
        asset_id: "legacy_asset".to_string(),
        currency_code: CurrencyCode::Usd,
        gross_dividend_value_minor_units: dividend_amount_minor,
        impure_income_ratio_bps: purification_ratio_bps,
        rounding_rule: PurificationRoundingRule::Floor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::{
        Asset, AssetIdentity, AssetProvider, AssetStatus, AssetType, Network, ProviderConfig,
        TokenDetails,
    };
    use crate::shariah::ownership::OwnershipRecord;
    use crate::shariah::policy::ScreeningPolicy;
    use crate::shariah::registry::register_asset_at;
    use crate::shariah::screening::{
        BusinessActivityAssessment, BusinessCategory, ShariahFinancialMetrics,
    };
    use crate::shariah::types::ShariahStatus;

    #[test]
    fn test_concept_separation_ratio_bps_vs_value_minor_units() {
        // Concept 1: impure_income_ratio_bps is a dimensionless fraction in basis points (120 bps = 1.20%).
        let impure_income_ratio_bps = 120u32;

        // Concept 2: gross dividend value in minor units (e.g. cents).
        // A gross dividend of $10,000.00 is 1,000,000 cents.
        let gross_dividend_value_minor_units = 1_000_000u64;

        let assessment = assess_dividend_purification(&DividendPurificationRequest {
            asset_id: "backed:NVDA".to_string(),
            currency_code: CurrencyCode::Usd,
            gross_dividend_value_minor_units,
            impure_income_ratio_bps,
            rounding_rule: PurificationRoundingRule::ConservativeCeiling,
        })
        .expect("Purification succeeds");

        // The purification payment value is 12,000 cents ($120.00).
        assert_eq!(assessment.purification_value_minor_units, 12_000);

        // CRITICAL INVARIANT: The ratio itself (120) is NOT the payment value (12,000)!
        assert_ne!(
            assessment.purification_value_minor_units, impure_income_ratio_bps as u64,
            "Purification payment amount must never equal the dimensionless ratio itself!"
        );

        assert_eq!(assessment.net_permissible_value_minor_units, 988_000);
        assert!(assessment.purification_required);
        assert!(!assessment.is_pure());

        // Test with another dividend size ($500.00 = 50,000 cents):
        // 120 bps = 1.20% of 50,000 cents = 600 cents ($6.00).
        let small_dividend_cents = 50_000u64;
        let small_calc = calculate_purification_value_minor_units(
            small_dividend_cents,
            impure_income_ratio_bps,
            PurificationRoundingRule::ConservativeCeiling,
        )
        .unwrap();
        assert_eq!(small_calc, 600);
        assert_ne!(small_calc, impure_income_ratio_bps as u64);
    }

    #[test]
    fn test_purification_independent_from_screening_approval() {
        // Screening policy requires impure_income_ratio_bps <= 500 bps (5.00%)
        let policy = ScreeningPolicy::board_approved_v1();
        let now = 1_705_000_000;

        let asset = Asset {
            identity: AssetIdentity {
                asset_id: "backed:NVDA".to_string(),
                symbol: "NVDA".to_string(),
                name: "NVIDIA Corporation".to_string(),
                asset_type: AssetType::Stock,
                underlying_reference: "US67066G1040".to_string(),
            },
            token: TokenDetails {
                mint: "So11111111111111111111111111111111111111112".to_string(),
                decimals: 6,
                network: Network::SolanaMainnet,
            },
            provider: ProviderConfig {
                provider: AssetProvider::Backed,
                price_feed_id: "0xnvdafeed".to_string(),
                meteora_pool: None,
                secondary_reference: None,
                status: AssetStatus::Active,
            },
        };

        let ownership = OwnershipRecord {
            verified: true,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:audit123".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_800_000_000,
        };

        let business = BusinessActivityAssessment::reviewed_permissible(
            BusinessCategory::Technology,
            "Semiconductor design and manufacturing",
            "SEC Form 10-K",
        );

        // 1. Asset has 150 bps impure income (1.50% <= 5.00% threshold)
        // Asset is fully APPROVED for trading:
        let approved_financials =
            ShariahFinancialMetrics::from_market_cap_ratios(1_500, 1_000, 150);
        let registered = register_asset_at(
            asset.clone(),
            ownership.clone(),
            approved_financials,
            business.clone(),
            &policy,
            now,
        )
        .expect("Asset passes screening");

        assert_eq!(registered.eligibility.status, ShariahStatus::Approved);
        assert!(registered.is_tradeable(now));

        // 2. Post-settlement dividend distribution of $20,000.00 (2,000,000 cents):
        // Purification is evaluated independently:
        let purification = assess_purification_for_registered_asset(
            &registered,
            2_000_000,
            CurrencyCode::Usd,
            PurificationRoundingRule::ConservativeCeiling,
        )
        .expect("Purification calculated successfully");

        assert_eq!(purification.gross_dividend_value_minor_units, 2_000_000);
        assert_eq!(purification.impure_income_ratio_bps, 150);
        // 1.50% of 2,000,000 cents = 30,000 cents ($300.00)
        assert_eq!(purification.purification_value_minor_units, 30_000);
        assert_eq!(purification.net_permissible_value_minor_units, 1_970_000);
        assert!(purification.purification_required);

        // 3. Clean company with 0 bps impure income:
        let clean_financials = ShariahFinancialMetrics::from_market_cap_ratios(1_500, 1_000, 0);
        let mut clean_asset = asset.clone();
        clean_asset.identity.asset_id = "backed:CLEAN".to_string();
        let registered_clean = register_asset_at(
            clean_asset,
            ownership.clone(),
            clean_financials,
            business.clone(),
            &policy,
            now,
        )
        .unwrap();

        let clean_purification = assess_purification_for_registered_asset(
            &registered_clean,
            2_000_000,
            CurrencyCode::Usd,
            PurificationRoundingRule::ConservativeCeiling,
        )
        .unwrap();

        assert_eq!(clean_purification.purification_value_minor_units, 0);
        assert_eq!(
            clean_purification.net_permissible_value_minor_units,
            2_000_000
        );
        assert!(clean_purification.is_pure());
        assert!(!clean_purification.purification_required);
    }

    #[test]
    fn test_rounding_rules_exact_and_fractional_boundaries() {
        // Case 1: Exact integer division (zero remainder)
        // Gross = 10,000 cents ($100.00), Ratio = 100 bps (1.00%)
        // 10,000 * 100 / 10,000 = 100 cents exact
        let exact_gross = 10_000u64;
        let exact_ratio = 100u32;
        assert_eq!(
            calculate_purification_value_minor_units(
                exact_gross,
                exact_ratio,
                PurificationRoundingRule::Floor
            )
            .unwrap(),
            100
        );
        assert_eq!(
            calculate_purification_value_minor_units(
                exact_gross,
                exact_ratio,
                PurificationRoundingRule::NearestHalfUp
            )
            .unwrap(),
            100
        );
        assert_eq!(
            calculate_purification_value_minor_units(
                exact_gross,
                exact_ratio,
                PurificationRoundingRule::ConservativeCeiling
            )
            .unwrap(),
            100
        );

        // Case 2: Fractional remainder strictly less than 0.5 (< 5,000 / 10,000)
        // Gross = 100 cents ($1.00), Ratio = 33 bps (0.33%)
        // Numerator = 3,300 -> quotient = 0, remainder = 3,300 (0.33 cents)
        let frac_low_gross = 100u64;
        let frac_low_ratio = 33u32;
        // Floor discards remainder -> 0
        assert_eq!(
            calculate_purification_value_minor_units(
                frac_low_gross,
                frac_low_ratio,
                PurificationRoundingRule::Floor
            )
            .unwrap(),
            0
        );
        // NearestHalfUp: 3,300 * 2 = 6,600 < 10,000 -> rounds down to 0
        assert_eq!(
            calculate_purification_value_minor_units(
                frac_low_gross,
                frac_low_ratio,
                PurificationRoundingRule::NearestHalfUp
            )
            .unwrap(),
            0
        );
        // ConservativeCeiling: remainder > 0 -> rounds UP to 1 cent (Islamic precaution)
        assert_eq!(
            calculate_purification_value_minor_units(
                frac_low_gross,
                frac_low_ratio,
                PurificationRoundingRule::ConservativeCeiling
            )
            .unwrap(),
            1
        );

        // Case 3: Fractional remainder exactly equal to 0.5 (= 5,000 / 10,000)
        // Gross = 100 cents ($1.00), Ratio = 50 bps (0.50%)
        // Numerator = 5,000 -> quotient = 0, remainder = 5,000 (0.50 cents)
        let half_gross = 100u64;
        let half_ratio = 50u32;
        // Floor -> 0
        assert_eq!(
            calculate_purification_value_minor_units(
                half_gross,
                half_ratio,
                PurificationRoundingRule::Floor
            )
            .unwrap(),
            0
        );
        // NearestHalfUp: 5,000 * 2 = 10,000 >= 10,000 -> rounds UP to 1 cent
        assert_eq!(
            calculate_purification_value_minor_units(
                half_gross,
                half_ratio,
                PurificationRoundingRule::NearestHalfUp
            )
            .unwrap(),
            1
        );
        // ConservativeCeiling -> rounds UP to 1 cent
        assert_eq!(
            calculate_purification_value_minor_units(
                half_gross,
                half_ratio,
                PurificationRoundingRule::ConservativeCeiling
            )
            .unwrap(),
            1
        );

        // Case 4: Fractional remainder strictly greater than 0.5
        // Gross = 1,000 cents ($10.00), Ratio = 75 bps (0.75%)
        // Numerator = 75,000 -> quotient = 7, remainder = 5,000 (7.50 cents)
        let frac_high_gross = 1_000u64;
        let frac_high_ratio = 75u32;
        assert_eq!(
            calculate_purification_value_minor_units(
                frac_high_gross,
                frac_high_ratio,
                PurificationRoundingRule::Floor
            )
            .unwrap(),
            7
        );
        assert_eq!(
            calculate_purification_value_minor_units(
                frac_high_gross,
                frac_high_ratio,
                PurificationRoundingRule::NearestHalfUp
            )
            .unwrap(),
            8
        );
        assert_eq!(
            calculate_purification_value_minor_units(
                frac_high_gross,
                frac_high_ratio,
                PurificationRoundingRule::ConservativeCeiling
            )
            .unwrap(),
            8
        );
    }

    #[test]
    fn test_zero_and_boundary_conditions() {
        // Zero gross dividend yields 0
        let zero_gross = calculate_purification_value_minor_units(
            0,
            120,
            PurificationRoundingRule::ConservativeCeiling,
        )
        .unwrap();
        assert_eq!(zero_gross, 0);

        // Zero ratio yields 0
        let zero_ratio = calculate_purification_value_minor_units(
            1_000_000,
            0,
            PurificationRoundingRule::ConservativeCeiling,
        )
        .unwrap();
        assert_eq!(zero_ratio, 0);

        // 10,000 bps (100% impure)
        let full_impure = calculate_purification_value_minor_units(
            50_000,
            10_000,
            PurificationRoundingRule::ConservativeCeiling,
        )
        .unwrap();
        assert_eq!(full_impure, 50_000);

        // Ratio > 10,000 bps errors
        let invalid_ratio = calculate_purification_value_minor_units(
            50_000,
            10_001,
            PurificationRoundingRule::ConservativeCeiling,
        );
        assert!(matches!(
            invalid_ratio,
            Err(PurificationError::RatioExceedsMaximum {
                impure_income_ratio_bps: 10_001
            })
        ));
    }

    #[test]
    fn test_per_share_dividend_purification() {
        // Investor holds 5,000 shares of an equity token
        // Company declares a dividend of $1.50 per share (150 cents)
        // Total gross dividend = 5,000 * 150 = 750,000 cents ($7,500.00)
        // Impure income ratio = 80 bps (0.80%)
        let req = PerShareDividendPurificationRequest {
            asset_id: "backed:AAPL".to_string(),
            currency_code: CurrencyCode::Usd,
            total_shares_count: 5_000,
            dividend_per_share_value_minor_units: 150,
            impure_income_ratio_bps: 80,
            rounding_rule: PurificationRoundingRule::ConservativeCeiling,
        };

        let assessment =
            assess_per_share_dividend_purification(&req).expect("Per-share purification succeeds");

        assert_eq!(assessment.gross_dividend_value_minor_units, 750_000);
        assert_eq!(assessment.impure_income_ratio_bps, 80);
        // 0.80% of 750,000 cents = 6,000 cents ($60.00)
        assert_eq!(assessment.purification_value_minor_units, 6_000);
        assert_eq!(assessment.net_permissible_value_minor_units, 744_000);
        assert!(!assessment.is_pure());
    }

    #[test]
    fn test_direct_impure_value_purification() {
        // Custodian reports total dividend of $5,000.00 (500,000 cents)
        // with an explicit non-permissible interest income breakdown of $60.00 (6,000 cents)
        let req = DirectValuePurificationRequest {
            asset_id: "backed:SPYx".to_string(),
            currency_code: CurrencyCode::Usdc,
            gross_dividend_value_minor_units: 500_000,
            impure_income_value_minor_units: 6_000,
        };

        let assessment = assess_direct_impure_value(&req).expect("Direct assessment succeeds");

        assert_eq!(assessment.gross_dividend_value_minor_units, 500_000);
        assert_eq!(assessment.purification_value_minor_units, 6_000);
        assert_eq!(assessment.net_permissible_value_minor_units, 494_000);
        // Effective ratio: 6,000 / 500,000 = 1.20% = 120 bps
        assert_eq!(assessment.impure_income_ratio_bps, 120);
        assert!(assessment.purification_required);

        // Impure value exceeding gross dividend returns error
        let invalid_req = DirectValuePurificationRequest {
            asset_id: "backed:SPYx".to_string(),
            currency_code: CurrencyCode::Usdc,
            gross_dividend_value_minor_units: 500_000,
            impure_income_value_minor_units: 600_000,
        };
        let err = assess_direct_impure_value(&invalid_req);
        assert!(matches!(
            err,
            Err(PurificationError::DirectValueExceedsGross {
                impure_income_value_minor_units: 600_000,
                gross_dividend_value_minor_units: 500_000,
            })
        ));
    }

    #[test]
    fn test_explicit_currency_units_and_decimals() {
        assert_eq!(CurrencyCode::Usdc.decimals(), 6);
        assert_eq!(CurrencyCode::Usd.decimals(), 2);
        assert_eq!(CurrencyCode::Eur.decimals(), 2);
        assert_eq!(CurrencyCode::Gbp.decimals(), 2);
        assert_eq!(CurrencyCode::Sol.decimals(), 9);
        assert_eq!(CurrencyCode::Custom("AED".to_string()).decimals(), 2);

        assert_eq!(CurrencyCode::Usdc.to_string(), "USDC");
        assert_eq!(CurrencyCode::Usd.to_string(), "USD");
        assert_eq!(CurrencyCode::Sol.to_string(), "SOL");
        assert_eq!(CurrencyCode::Custom("MYR".to_string()).to_string(), "MYR");
    }

    #[test]
    fn test_purification_assessment_serde_roundtrip() {
        let assessment = PurificationAssessment {
            asset_id: "backed:NVDA".to_string(),
            currency_code: CurrencyCode::Usdc,
            gross_dividend_value_minor_units: 1_000_000,
            impure_income_ratio_bps: 120,
            purification_value_minor_units: 12_000,
            net_permissible_value_minor_units: 988_000,
            rounding_rule: PurificationRoundingRule::ConservativeCeiling,
            purification_required: true,
        };

        let json = serde_json::to_string(&assessment).expect("Serialization succeeds");
        let deserialized: PurificationAssessment =
            serde_json::from_str(&json).expect("Deserialization succeeds");

        assert_eq!(deserialized, assessment);
        assert_eq!(deserialized.currency_code(), &CurrencyCode::Usdc);
        assert_eq!(
            deserialized.rounding_rule(),
            PurificationRoundingRule::ConservativeCeiling
        );
        assert_eq!(deserialized.purification_value_minor_units(), 12_000);
        assert_eq!(deserialized.net_permissible_value_minor_units(), 988_000);
        assert_eq!(deserialized.gross_dividend_value_minor_units(), 1_000_000);
        assert_eq!(deserialized.impure_income_ratio_bps(), 120);
    }
}
