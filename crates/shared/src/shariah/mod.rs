//! Shariah screening and governance domain module.
//!
//! # Architecture & Governance
//! This module encapsulates the core Shariah domain logic for Equity Catalyst:
//! - [`ShariahStatus`]: Canonical states (`Pending`, `Approved`, `Rejected`, `Expired`, `Revoked`).
//! - [`ScreeningStandard`]: Decoupled standards (`BoardApprovedV1`, `Aaoifi21`, `Custom(String)`).
//! - [`ShariahRejectionReason`]: Deterministic classification of non-compliance grounds.
//! - [`ScreeningPolicy`]: Versioned, parameter-driven threshold policies.
//! - [`evaluate_shariah_compliance`]: Pure, deterministic evaluation of assets against policy.
//!
//! # Regulatory Distinction
//! Software-based algorithmic screening verifies adherence to configured mathematical
//! and qualitative criteria. It does not replace or constitute formal certification
//! by a qualified Shariah supervisory board.

pub mod eligibility;
pub mod ownership;
pub mod policy;
pub mod registry;
pub mod screening;
pub mod types;
pub mod validation;

pub use eligibility::*;
pub use ownership::*;
pub use policy::*;
pub use registry::*;
pub use screening::*;
pub use types::*;
pub use validation::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BasisPoints;
    use crate::validation::ValidationError;

    #[test]
    fn test_shariah_status_serialization_and_methods() {
        let statuses = vec![
            (ShariahStatus::Pending, "\"Pending\""),
            (ShariahStatus::Approved, "\"Approved\""),
            (ShariahStatus::Rejected, "\"Rejected\""),
            (ShariahStatus::Expired, "\"Expired\""),
            (ShariahStatus::Revoked, "\"Revoked\""),
        ];

        for (status, expected_json) in statuses {
            let serialized = serde_json::to_string(&status).expect("Serialization failed");
            assert_eq!(serialized, expected_json);

            let deserialized: ShariahStatus =
                serde_json::from_str(&serialized).expect("Deserialization failed");
            assert_eq!(deserialized, status);
        }

        // Test status helper methods
        assert!(ShariahStatus::Approved.is_approved());
        assert!(ShariahStatus::Approved.can_trade());
        assert!(!ShariahStatus::Approved.is_rejected());

        assert!(ShariahStatus::Pending.is_pending());
        assert!(!ShariahStatus::Pending.can_trade());

        assert!(ShariahStatus::Rejected.is_rejected());
        assert!(!ShariahStatus::Rejected.can_trade());

        assert!(ShariahStatus::Expired.is_expired());
        assert!(!ShariahStatus::Expired.can_trade());

        assert!(ShariahStatus::Revoked.is_revoked());
        assert!(!ShariahStatus::Revoked.can_trade());
    }

    #[test]
    fn test_screening_standard_serialization() {
        let standards = vec![
            (ScreeningStandard::BoardApprovedV1, "\"BoardApprovedV1\""),
            (ScreeningStandard::Aaoifi21, "\"Aaoifi21\""),
            (
                ScreeningStandard::Custom("DJIM".to_string()),
                "{\"Custom\":\"DJIM\"}",
            ),
            (
                ScreeningStandard::Custom("SAC-SC-Malaysia".to_string()),
                "{\"Custom\":\"SAC-SC-Malaysia\"}",
            ),
        ];

        for (standard, expected_json) in standards {
            let serialized = serde_json::to_string(&standard).expect("Serialization failed");
            assert_eq!(serialized, expected_json);

            let deserialized: ScreeningStandard =
                serde_json::from_str(&serialized).expect("Deserialization failed");
            assert_eq!(deserialized, standard);
        }
    }

    #[test]
    fn test_shariah_rejection_reason_serialization() {
        let reasons = vec![
            (
                ShariahRejectionReason::ProhibitedBusiness,
                "\"ProhibitedBusiness\"",
            ),
            (ShariahRejectionReason::ExcessDebt, "\"ExcessDebt\""),
            (
                ShariahRejectionReason::ExcessInterestBearingCash,
                "\"ExcessInterestBearingCash\"",
            ),
            (
                ShariahRejectionReason::ExcessImpureIncome,
                "\"ExcessImpureIncome\"",
            ),
            (
                ShariahRejectionReason::OwnershipUnverified,
                "\"OwnershipUnverified\"",
            ),
            (
                ShariahRejectionReason::SyntheticExposure,
                "\"SyntheticExposure\"",
            ),
            (
                ShariahRejectionReason::MissingEvidence,
                "\"MissingEvidence\"",
            ),
            (ShariahRejectionReason::ReviewExpired, "\"ReviewExpired\""),
            (
                ShariahRejectionReason::Other("Non-compliant asset rehypothecation".to_string()),
                "{\"Other\":\"Non-compliant asset rehypothecation\"}",
            ),
        ];

        for (reason, expected_json) in reasons {
            let serialized = serde_json::to_string(&reason).expect("Serialization failed");
            assert_eq!(serialized, expected_json);

            let deserialized: ShariahRejectionReason =
                serde_json::from_str(&serialized).expect("Deserialization failed");
            assert_eq!(deserialized, reason);
        }
    }

    #[test]
    fn test_screening_policy_construction() {
        // Board Approved V1 default
        let board_policy = ScreeningPolicy::board_approved_v1();
        assert_eq!(board_policy.standard, ScreeningStandard::BoardApprovedV1);
        assert_eq!(board_policy.policy_version, "v1.0");
        assert_eq!(board_policy.debt_limit_bps, BasisPoints(3_000));
        assert_eq!(
            board_policy.interest_bearing_cash_limit_bps,
            BasisPoints(3_000)
        );
        assert_eq!(board_policy.impure_income_limit_bps, BasisPoints(500));
        assert!(board_policy.validate().is_ok());

        // AAOIFI 21 parameterized
        let aaoifi = ScreeningPolicy::aaoifi_21("2024.2");
        assert_eq!(aaoifi.standard, ScreeningStandard::Aaoifi21);
        assert_eq!(aaoifi.policy_version, "2024.2");
        assert_eq!(aaoifi.debt_limit_bps, BasisPoints(3_000));

        // Custom standard with alternative 33% debt threshold (e.g. DJIM)
        let custom_policy = ScreeningPolicy::custom(
            "DJIM",
            "djim-2024",
            BasisPoints(3_300),
            BasisPoints(3_300),
            BasisPoints(500),
            ThresholdComparison::LessThanOrEqual,
        )
        .expect("Failed to build custom policy");

        assert_eq!(
            custom_policy.standard,
            ScreeningStandard::Custom("DJIM".to_string())
        );
        assert_eq!(custom_policy.debt_limit_bps, BasisPoints(3_300));
        assert_eq!(
            custom_policy.interest_bearing_cash_limit_bps,
            BasisPoints(3_300)
        );
        assert_eq!(
            custom_policy.comparison,
            ThresholdComparison::LessThanOrEqual
        );

        // Construction via from_raw_bps
        let raw_policy = ScreeningPolicy::from_raw_bps(
            ScreeningStandard::BoardApprovedV1,
            "raw-v1",
            2_500,
            2_500,
            300,
            ThresholdComparison::StrictLessThan,
        )
        .expect("Valid raw policy");
        assert_eq!(raw_policy.debt_limit_bps, BasisPoints(2_500));
        assert_eq!(raw_policy.comparison, ThresholdComparison::StrictLessThan);
    }

    #[test]
    fn test_screening_policy_validation_boundaries() {
        // Invalid debt bps (> 10,000)
        let invalid_debt = ScreeningPolicy::new(
            ScreeningStandard::BoardApprovedV1,
            "v1",
            BasisPoints(10_001),
            BasisPoints(3_000),
            BasisPoints(500),
            ThresholdComparison::LessThanOrEqual,
        );
        assert!(matches!(
            invalid_debt,
            Err(ValidationError::InvalidBasisPoints(10_001))
        ));

        // Empty policy version
        let invalid_version = ScreeningPolicy::new(
            ScreeningStandard::BoardApprovedV1,
            "   ",
            BasisPoints(3_000),
            BasisPoints(3_000),
            BasisPoints(500),
            ThresholdComparison::LessThanOrEqual,
        );
        assert!(matches!(
            invalid_version,
            Err(ValidationError::InvalidParam(_))
        ));

        // Empty custom standard name
        let invalid_custom = ScreeningPolicy::custom(
            "",
            "v1",
            BasisPoints(3_000),
            BasisPoints(3_000),
            BasisPoints(500),
            ThresholdComparison::LessThanOrEqual,
        );
        assert!(matches!(
            invalid_custom,
            Err(ValidationError::InvalidParam(_))
        ));
    }

    #[test]
    fn test_screening_policy_serde() {
        let policy = ScreeningPolicy::board_approved_v1();
        let json = serde_json::to_string(&policy).expect("Serialize policy");
        let parsed: ScreeningPolicy = serde_json::from_str(&json).expect("Deserialize policy");
        assert_eq!(policy, parsed);
    }

    #[test]
    fn test_compliance_evaluation_clean_asset_approved() {
        let policy = ScreeningPolicy::board_approved_v1();
        let input = ScreeningAssessmentInput {
            has_permissible_business: true,
            is_ownership_verified: true,
            is_synthetic: false,
            has_sufficient_evidence: true,
            is_review_current: true,
            debt_ratio_bps: BasisPoints(1_500), // 15% < 30%
            interest_bearing_cash_ratio_bps: BasisPoints(1_000), // 10% < 30%
            impure_income_ratio_bps: BasisPoints(100), // 1% < 5%
        };

        let (status, reasons) = evaluate_shariah_compliance(&policy, &input);
        assert_eq!(status, ShariahStatus::Approved);
        assert!(reasons.is_empty());
    }

    #[test]
    fn test_compliance_evaluation_decoupled_standards_debt_threshold() {
        // Asset has 32% interest-bearing debt
        let input = ScreeningAssessmentInput {
            debt_ratio_bps: BasisPoints(3_200), // 32.00%
            interest_bearing_cash_ratio_bps: BasisPoints(1_000),
            impure_income_ratio_bps: BasisPoints(100),
            ..Default::default()
        };

        // Under AAOIFI Standard 21 (30% limit), this must be REJECTED with ExcessDebt
        let aaoifi_policy = ScreeningPolicy::aaoifi_21("2024");
        let (status_aaoifi, reasons_aaoifi) = evaluate_shariah_compliance(&aaoifi_policy, &input);
        assert_eq!(status_aaoifi, ShariahStatus::Rejected);
        assert_eq!(reasons_aaoifi, vec![ShariahRejectionReason::ExcessDebt]);

        // Under a 33% custom board standard (e.g. DJIM benchmark), this is APPROVED
        let custom_policy = ScreeningPolicy::custom(
            "DJIM",
            "2024",
            BasisPoints(3_300),
            BasisPoints(3_300),
            BasisPoints(500),
            ThresholdComparison::LessThanOrEqual,
        )
        .unwrap();

        let (status_custom, reasons_custom) = evaluate_shariah_compliance(&custom_policy, &input);
        assert_eq!(status_custom, ShariahStatus::Approved);
        assert!(reasons_custom.is_empty());
    }

    #[test]
    fn test_compliance_evaluation_synthetic_and_prohibited_business() {
        let policy = ScreeningPolicy::board_approved_v1();

        // Synthetic exposure failure
        let synthetic_input = ScreeningAssessmentInput {
            is_synthetic: true,
            ..Default::default()
        };
        let (status, reasons) = evaluate_shariah_compliance(&policy, &synthetic_input);
        assert_eq!(status, ShariahStatus::Rejected);
        assert!(reasons.contains(&ShariahRejectionReason::SyntheticExposure));

        // Prohibited business sector failure
        let prohibited_input = ScreeningAssessmentInput {
            has_permissible_business: false,
            ..Default::default()
        };
        let (status, reasons) = evaluate_shariah_compliance(&policy, &prohibited_input);
        assert_eq!(status, ShariahStatus::Rejected);
        assert!(reasons.contains(&ShariahRejectionReason::ProhibitedBusiness));

        // Unverified ownership failure
        let unverified_input = ScreeningAssessmentInput {
            is_ownership_verified: false,
            ..Default::default()
        };
        let (status, reasons) = evaluate_shariah_compliance(&policy, &unverified_input);
        assert_eq!(status, ShariahStatus::Rejected);
        assert!(reasons.contains(&ShariahRejectionReason::OwnershipUnverified));
    }

    #[test]
    fn test_compliance_evaluation_pending_and_expired_states() {
        let policy = ScreeningPolicy::board_approved_v1();

        // Missing evidence without substantive violations results in Pending
        let pending_input = ScreeningAssessmentInput {
            has_sufficient_evidence: false,
            ..Default::default()
        };
        let (status, reasons) = evaluate_shariah_compliance(&policy, &pending_input);
        assert_eq!(status, ShariahStatus::Pending);
        assert_eq!(reasons, vec![ShariahRejectionReason::MissingEvidence]);

        // Expired review without substantive violations results in Expired
        let expired_input = ScreeningAssessmentInput {
            is_review_current: false,
            ..Default::default()
        };
        let (status, reasons) = evaluate_shariah_compliance(&policy, &expired_input);
        assert_eq!(status, ShariahStatus::Expired);
        assert_eq!(reasons, vec![ShariahRejectionReason::ReviewExpired]);

        // If an asset has both an expired review AND excess debt, substantive failure takes precedence -> Rejected
        let expired_and_excess_debt = ScreeningAssessmentInput {
            is_review_current: false,
            debt_ratio_bps: BasisPoints(4_000), // 40%
            ..Default::default()
        };
        let (status, reasons) = evaluate_shariah_compliance(&policy, &expired_and_excess_debt);
        assert_eq!(status, ShariahStatus::Rejected);
        assert!(reasons.contains(&ShariahRejectionReason::ExcessDebt));
        assert!(reasons.contains(&ShariahRejectionReason::ReviewExpired));
    }

    #[test]
    fn test_multiple_concurrent_rejections() {
        let policy = ScreeningPolicy::board_approved_v1();
        let failing_input = ScreeningAssessmentInput {
            has_permissible_business: false,
            is_synthetic: true,
            is_ownership_verified: false,
            debt_ratio_bps: BasisPoints(5_000),
            interest_bearing_cash_ratio_bps: BasisPoints(4_000),
            impure_income_ratio_bps: BasisPoints(1_000),
            has_sufficient_evidence: false,
            is_review_current: false,
        };

        let (status, reasons) = evaluate_shariah_compliance(&policy, &failing_input);
        assert_eq!(status, ShariahStatus::Rejected);
        assert_eq!(reasons.len(), 8);
        assert!(reasons.contains(&ShariahRejectionReason::ProhibitedBusiness));
        assert!(reasons.contains(&ShariahRejectionReason::SyntheticExposure));
        assert!(reasons.contains(&ShariahRejectionReason::OwnershipUnverified));
        assert!(reasons.contains(&ShariahRejectionReason::ExcessDebt));
        assert!(reasons.contains(&ShariahRejectionReason::ExcessInterestBearingCash));
        assert!(reasons.contains(&ShariahRejectionReason::ExcessImpureIncome));
        assert!(reasons.contains(&ShariahRejectionReason::MissingEvidence));
        assert!(reasons.contains(&ShariahRejectionReason::ReviewExpired));
    }

    #[test]
    fn test_shariah_eligibility_approved_valid_asset() {
        let eligibility = ShariahEligibility {
            status: ShariahStatus::Approved,
            standard: ScreeningStandard::BoardApprovedV1,
            business_activity_approved: true,
            debt_ratio_bps: 1_500,            // 15% < 30%
            interest_bearing_cash_bps: 1_200, // 12% < 30%
            impure_income_bps: 200,           // 2% < 5%
            ownership_verified: true,
            evidence_hash:
                "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
            reviewed_at: 1_700_000_000,
            expires_at: 1_710_000_000,
            policy_version: "v1.0".to_string(),
        };

        let now = 1_705_000_000;
        assert!(eligibility.is_currently_eligible(now));
        assert!(eligibility.validate_eligibility(now).is_ok());
    }

    #[test]
    fn test_shariah_eligibility_rejected_status() {
        let eligibility = ShariahEligibility {
            status: ShariahStatus::Rejected,
            standard: ScreeningStandard::BoardApprovedV1,
            business_activity_approved: true,
            debt_ratio_bps: 1_500,
            interest_bearing_cash_bps: 1_200,
            impure_income_bps: 200,
            ownership_verified: true,
            evidence_hash: "sha256:test".to_string(),
            reviewed_at: 1_700_000_000,
            expires_at: 1_710_000_000,
            policy_version: "v1.0".to_string(),
        };

        let now = 1_705_000_000;
        assert!(!eligibility.is_currently_eligible(now));
        let res = eligibility.validate_eligibility(now);
        assert!(res.is_err());
        let reasons = res.unwrap_err();
        assert!(reasons
            .iter()
            .any(|r| matches!(r, ShariahRejectionReason::Other(msg) if msg.contains("Rejected"))));
    }

    #[test]
    fn test_shariah_eligibility_expired_asset() {
        let eligibility = ShariahEligibility {
            status: ShariahStatus::Approved,
            standard: ScreeningStandard::BoardApprovedV1,
            business_activity_approved: true,
            debt_ratio_bps: 1_500,
            interest_bearing_cash_bps: 1_200,
            impure_income_bps: 200,
            ownership_verified: true,
            evidence_hash: "sha256:test".to_string(),
            reviewed_at: 1_700_000_000,
            expires_at: 1_710_000_000,
            policy_version: "v1.0".to_string(),
        };

        // Timestamp is past expiration
        let now = 1_710_000_001;
        assert!(!eligibility.is_currently_eligible(now));
        let reasons = eligibility.validate_eligibility(now).unwrap_err();
        assert!(reasons.contains(&ShariahRejectionReason::ReviewExpired));

        // Timestamp is before reviewed_at (clock skew or future record)
        let early_now = 1_699_999_999;
        assert!(!eligibility.is_currently_eligible(early_now));
    }

    #[test]
    fn test_shariah_eligibility_ownership_false() {
        let eligibility = ShariahEligibility {
            status: ShariahStatus::Approved,
            standard: ScreeningStandard::BoardApprovedV1,
            business_activity_approved: true,
            debt_ratio_bps: 1_500,
            interest_bearing_cash_bps: 1_200,
            impure_income_bps: 200,
            ownership_verified: false, // Inability to verify SPV / custody
            evidence_hash: "sha256:test".to_string(),
            reviewed_at: 1_700_000_000,
            expires_at: 1_710_000_000,
            policy_version: "v1.0".to_string(),
        };

        let now = 1_705_000_000;
        assert!(!eligibility.is_currently_eligible(now));
        let reasons = eligibility.validate_eligibility(now).unwrap_err();
        assert!(reasons.contains(&ShariahRejectionReason::OwnershipUnverified));
    }

    #[test]
    fn test_shariah_eligibility_prohibited_business() {
        let eligibility = ShariahEligibility {
            status: ShariahStatus::Approved,
            standard: ScreeningStandard::BoardApprovedV1,
            business_activity_approved: false, // Prohibited core business
            debt_ratio_bps: 1_000,
            interest_bearing_cash_bps: 1_000,
            impure_income_bps: 100,
            ownership_verified: true,
            evidence_hash: "sha256:test".to_string(),
            reviewed_at: 1_700_000_000,
            expires_at: 1_710_000_000,
            policy_version: "v1.0".to_string(),
        };

        let now = 1_705_000_000;
        assert!(!eligibility.is_currently_eligible(now));
        let reasons = eligibility.validate_eligibility(now).unwrap_err();
        assert!(reasons.contains(&ShariahRejectionReason::ProhibitedBusiness));
    }

    #[test]
    fn test_shariah_eligibility_debt_threshold_failure() {
        let eligibility = ShariahEligibility {
            status: ShariahStatus::Approved,
            standard: ScreeningStandard::Aaoifi21,
            business_activity_approved: true,
            debt_ratio_bps: 3_200, // 32.00% > 30.00% AAOIFI limit
            interest_bearing_cash_bps: 1_000,
            impure_income_bps: 100,
            ownership_verified: true,
            evidence_hash: "sha256:test".to_string(),
            reviewed_at: 1_700_000_000,
            expires_at: 1_710_000_000,
            policy_version: "aaoifi-2024".to_string(),
        };

        let now = 1_705_000_000;
        // Under AAOIFI default policy (30%), it fails
        assert!(!eligibility.is_currently_eligible(now));
        let reasons = eligibility.validate_eligibility(now).unwrap_err();
        assert!(reasons.contains(&ShariahRejectionReason::ExcessDebt));

        // Under custom policy permitting 33% debt, it passes
        let custom_policy = ScreeningPolicy::custom(
            "DJIM",
            "djim-2024",
            BasisPoints(3_300),
            BasisPoints(3_300),
            BasisPoints(500),
            ThresholdComparison::LessThanOrEqual,
        )
        .unwrap();
        assert!(eligibility.is_currently_eligible_under_policy(&custom_policy, now));
    }

    #[test]
    fn test_shariah_eligibility_impure_income_threshold_failure() {
        let eligibility = ShariahEligibility {
            status: ShariahStatus::Approved,
            standard: ScreeningStandard::BoardApprovedV1,
            business_activity_approved: true,
            debt_ratio_bps: 2_000,
            interest_bearing_cash_bps: 1_500,
            impure_income_bps: 650, // 6.50% > 5.00% limit
            ownership_verified: true,
            evidence_hash: "sha256:test".to_string(),
            reviewed_at: 1_700_000_000,
            expires_at: 1_710_000_000,
            policy_version: "v1.0".to_string(),
        };

        let now = 1_705_000_000;
        assert!(!eligibility.is_currently_eligible(now));
        let reasons = eligibility.validate_eligibility(now).unwrap_err();
        assert!(reasons.contains(&ShariahRejectionReason::ExcessImpureIncome));
    }

    #[test]
    fn test_shariah_eligibility_empty_evidence_hash() {
        let eligibility = ShariahEligibility {
            status: ShariahStatus::Approved,
            standard: ScreeningStandard::BoardApprovedV1,
            business_activity_approved: true,
            debt_ratio_bps: 1_500,
            interest_bearing_cash_bps: 1_200,
            impure_income_bps: 200,
            ownership_verified: true,
            evidence_hash: "   ".to_string(), // Empty / whitespace hash
            reviewed_at: 1_700_000_000,
            expires_at: 1_710_000_000,
            policy_version: "v1.0".to_string(),
        };

        let now = 1_705_000_000;
        assert!(!eligibility.is_currently_eligible(now));
        let reasons = eligibility.validate_eligibility(now).unwrap_err();
        assert!(reasons.contains(&ShariahRejectionReason::MissingEvidence));
    }

    #[test]
    fn test_shariah_eligibility_serde() {
        let eligibility = ShariahEligibility {
            status: ShariahStatus::Approved,
            standard: ScreeningStandard::Aaoifi21,
            business_activity_approved: true,
            debt_ratio_bps: 2_100,
            interest_bearing_cash_bps: 1_400,
            impure_income_bps: 150,
            ownership_verified: true,
            evidence_hash: "sha256:abcdef0123456789".to_string(),
            reviewed_at: 1_700_000_000,
            expires_at: 1_710_000_000,
            policy_version: "2024.1".to_string(),
        };

        let json = serde_json::to_string(&eligibility).expect("Serialize eligibility");
        let parsed: ShariahEligibility =
            serde_json::from_str(&json).expect("Deserialize eligibility");
        assert_eq!(eligibility, parsed);
    }

    #[test]
    fn test_ownership_record_valid_construction_and_currency() {
        let record = OwnershipRecord {
            verified: true,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:d5c0b9...cert".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_710_000_000,
        };

        let now = 1_705_000_000;
        assert!(validate_ownership(&record).is_ok());
        assert!(validate_ownership_at(&record, now).is_ok());
        assert!(record.is_current(now));
        assert!(is_current(&record, now));
    }

    #[test]
    fn test_ownership_record_unverified_failure() {
        let record = OwnershipRecord {
            verified: false, // Counsel / audit not signed
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:d5c0b9...cert".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_710_000_000,
        };

        let now = 1_705_000_000;
        assert!(validate_ownership(&record).is_err());
        assert!(!record.is_current(now));
    }

    #[test]
    fn test_ownership_record_empty_fields_failure() {
        let base_record = OwnershipRecord {
            verified: true,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:d5c0b9...cert".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_710_000_000,
        };

        // Empty issuer
        let mut r = base_record.clone();
        r.issuer = "   ".to_string();
        assert!(validate_ownership(&r).is_err());

        // Empty custodian
        let mut r = base_record.clone();
        r.custodian = String::new();
        assert!(validate_ownership(&r).is_err());

        // Empty legal structure
        let mut r = base_record.clone();
        r.legal_structure = String::new();
        assert!(validate_ownership(&r).is_err());

        // Empty instrument reference
        let mut r = base_record.clone();
        r.instrument_reference = String::new();
        assert!(validate_ownership(&r).is_err());

        // Empty evidence hash
        let mut r = base_record.clone();
        r.evidence_hash = "  ".to_string();
        assert!(validate_ownership(&r).is_err());

        // Inverted or equal timestamps
        let mut r = base_record.clone();
        r.verified_at = 1_710_000_000;
        r.expires_at = 1_700_000_000;
        assert!(validate_ownership(&r).is_err());
    }

    #[test]
    fn test_ownership_record_temporal_boundaries() {
        let record = OwnershipRecord {
            verified: true,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:d5c0b9...cert".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_710_000_000,
        };

        // Timestamp expired
        let expired_now = 1_710_000_000;
        assert!(!record.is_current(expired_now));
        assert!(validate_ownership_at(&record, expired_now).is_err());

        // Timestamp before verified_at
        let early_now = 1_699_999_999;
        assert!(!record.is_current(early_now));
        assert!(validate_ownership_at(&record, early_now).is_err());

        // Valid timestamp boundary
        assert!(record.is_current(1_700_000_000));
        assert!(record.is_current(1_709_999_999));
    }

    #[test]
    fn test_ownership_record_serde() {
        let record = OwnershipRecord {
            verified: true,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:abcd".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_710_000_000,
        };

        let json = serde_json::to_string(&record).expect("Serialize ownership record");
        let parsed: OwnershipRecord =
            serde_json::from_str(&json).expect("Deserialize ownership record");
        assert_eq!(record, parsed);
    }

    #[test]
    fn test_screening_engine_threshold_inclusive_boundaries() {
        let policy = ScreeningPolicy::board_approved_v1(); // debt: 3,000 bps, cash: 3,000 bps, impure: 500 bps
        let valid_ownership = OwnershipRecord {
            verified: true,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:cert123".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_710_000_000,
        };
        let now = 1_705_000_000;

        // 1. Exactly equal to limit (inclusive threshold: passes)
        let exact_metrics = ShariahFinancialMetrics {
            debt_ratio_bps: 3_000,
            interest_bearing_cash_ratio_bps: 3_000,
            impure_income_ratio_bps: 500,
        };
        let res_exact = screen_asset(
            &BusinessCategory::Technology,
            &exact_metrics,
            &valid_ownership,
            &policy,
            now,
        );
        assert_eq!(res_exact, ScreeningResult::Approved);
        assert!(res_exact.is_approved());

        // 2. One unit below limit (passes)
        let below_metrics = ShariahFinancialMetrics {
            debt_ratio_bps: 2_999,
            interest_bearing_cash_ratio_bps: 2_999,
            impure_income_ratio_bps: 499,
        };
        let res_below = screen_asset(
            &BusinessCategory::Healthcare,
            &below_metrics,
            &valid_ownership,
            &policy,
            now,
        );
        assert_eq!(res_below, ScreeningResult::Approved);

        // 3. One unit above limit: Debt failure (3,001 bps)
        let debt_fail = ShariahFinancialMetrics {
            debt_ratio_bps: 3_001,
            interest_bearing_cash_ratio_bps: 1_000,
            impure_income_ratio_bps: 100,
        };
        let res_debt = screen_asset(
            &BusinessCategory::Technology,
            &debt_fail,
            &valid_ownership,
            &policy,
            now,
        );
        assert_eq!(
            res_debt,
            ScreeningResult::Rejected {
                reason: ShariahRejectionReason::ExcessDebt
            }
        );

        // 4. One unit above limit: Cash failure (3,001 bps)
        let cash_fail = ShariahFinancialMetrics {
            debt_ratio_bps: 1_000,
            interest_bearing_cash_ratio_bps: 3_001,
            impure_income_ratio_bps: 100,
        };
        let res_cash = screen_asset(
            &BusinessCategory::Manufacturing,
            &cash_fail,
            &valid_ownership,
            &policy,
            now,
        );
        assert_eq!(
            res_cash,
            ScreeningResult::Rejected {
                reason: ShariahRejectionReason::ExcessInterestBearingCash
            }
        );

        // 5. One unit above limit: Impure income failure (501 bps)
        let impure_fail = ShariahFinancialMetrics {
            debt_ratio_bps: 1_000,
            interest_bearing_cash_ratio_bps: 1_000,
            impure_income_ratio_bps: 501,
        };
        let res_impure = screen_asset(
            &BusinessCategory::Technology,
            &impure_fail,
            &valid_ownership,
            &policy,
            now,
        );
        assert_eq!(
            res_impure,
            ScreeningResult::Rejected {
                reason: ShariahRejectionReason::ExcessImpureIncome
            }
        );
    }

    #[test]
    fn test_screening_engine_prohibited_business_sectors() {
        let policy = ScreeningPolicy::board_approved_v1();
        let valid_ownership = OwnershipRecord {
            verified: true,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:cert123".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_710_000_000,
        };
        let clean_metrics = ShariahFinancialMetrics {
            debt_ratio_bps: 500,
            interest_bearing_cash_ratio_bps: 500,
            impure_income_ratio_bps: 50,
        };
        let now = 1_705_000_000;

        let prohibited_sectors = vec![
            BusinessCategory::ConventionalFinance,
            BusinessCategory::Alcohol,
            BusinessCategory::Gambling,
            BusinessCategory::Tobacco,
            BusinessCategory::Weapons,
            BusinessCategory::AdultEntertainment,
        ];

        for sector in prohibited_sectors {
            let res = screen_asset(&sector, &clean_metrics, &valid_ownership, &policy, now);
            assert_eq!(
                res,
                ScreeningResult::Rejected {
                    reason: ShariahRejectionReason::ProhibitedBusiness
                }
            );
        }

        // Permissible sectors
        let permissible_sectors = vec![
            BusinessCategory::Technology,
            BusinessCategory::Healthcare,
            BusinessCategory::Manufacturing,
            BusinessCategory::Other("Renewable Energy".to_string()),
        ];

        for sector in permissible_sectors {
            let res = screen_asset(&sector, &clean_metrics, &valid_ownership, &policy, now);
            assert_eq!(res, ScreeningResult::Approved);
        }
    }

    #[test]
    fn test_screening_engine_ownership_failures() {
        let policy = ScreeningPolicy::board_approved_v1();
        let clean_metrics = ShariahFinancialMetrics {
            debt_ratio_bps: 500,
            interest_bearing_cash_ratio_bps: 500,
            impure_income_ratio_bps: 50,
        };
        let now = 1_705_000_000;

        // 1. Unverified ownership
        let unverified_ownership = OwnershipRecord {
            verified: false,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:cert123".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_710_000_000,
        };
        let res_unverified = screen_asset(
            &BusinessCategory::Technology,
            &clean_metrics,
            &unverified_ownership,
            &policy,
            now,
        );
        assert_eq!(
            res_unverified,
            ScreeningResult::Rejected {
                reason: ShariahRejectionReason::OwnershipUnverified
            }
        );

        // 2. Expired ownership review
        let expired_ownership = OwnershipRecord {
            verified: true,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:cert123".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_704_000_000, // Expired before now (1_705_000_000)
        };
        let res_expired = screen_asset(
            &BusinessCategory::Technology,
            &clean_metrics,
            &expired_ownership,
            &policy,
            now,
        );
        assert_eq!(
            res_expired,
            ScreeningResult::Rejected {
                reason: ShariahRejectionReason::ReviewExpired
            }
        );
    }

    #[test]
    fn test_shariah_asset_registry_full_security_gate() {
        use crate::asset::verified_mainnet_assets;

        let nvda_asset = verified_mainnet_assets()
            .into_iter()
            .find(|a| a.symbol() == "NVDA")
            .expect("NVDA asset exists in canonical registry");

        let policy = ScreeningPolicy::board_approved_v1();
        let valid_ownership = OwnershipRecord {
            verified: true,
            issuer: "Backed Finance AG".to_string(),
            custodian: "Maerki Baumann & Co. AG".to_string(),
            legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
            instrument_reference: "ISIN: US67066G1040".to_string(),
            evidence_hash: "sha256:cert_nvda_verified".to_string(),
            verified_at: 1_700_000_000,
            expires_at: 1_710_000_000,
        };
        let clean_financials = ShariahFinancialMetrics {
            debt_ratio_bps: 1_500,                  // 15% < 30%
            interest_bearing_cash_ratio_bps: 1_200, // 12% < 30%
            impure_income_ratio_bps: 100,           // 1% < 5%
        };
        let now = 1_705_000_000;

        let mut registry = ShariahAssetRegistry::new();

        // 1. Valid asset registers successfully and is tradeable
        let registered = registry
            .register(
                nvda_asset.clone(),
                valid_ownership.clone(),
                clean_financials.clone(),
                BusinessCategory::Technology,
                &policy,
                now,
            )
            .expect("Registration succeeds");

        assert_eq!(registered.asset.symbol(), "NVDA");
        assert_eq!(registered.eligibility.status, ShariahStatus::Approved);
        assert!(registry.is_tradeable("NVDA", now));
        assert!(registered.is_tradeable(now));
        assert_eq!(get_eligibility(&registered).status, ShariahStatus::Approved);

        // Duplicate registration fails
        let dup_err = registry.register(
            nvda_asset.clone(),
            valid_ownership.clone(),
            clean_financials.clone(),
            BusinessCategory::Technology,
            &policy,
            now,
        );
        assert!(matches!(dup_err, Err(RegistryError::DuplicateAsset(_))));

        // 2. Missing/unverified ownership rejected
        let mut unverified_ownership = valid_ownership.clone();
        unverified_ownership.verified = false;
        let mut nvda_unverified = nvda_asset.clone();
        nvda_unverified.identity.asset_id = "backed:NVDA_unverified".to_string();

        let unverified_err = register_asset_at(
            nvda_unverified,
            unverified_ownership,
            clean_financials.clone(),
            BusinessCategory::Technology,
            &policy,
            now,
        );
        assert!(matches!(
            unverified_err,
            Err(RegistryError::InvalidOwnership(_))
        ));

        // 3. Prohibited business sector rejected
        let mut nvda_prohibited = nvda_asset.clone();
        nvda_prohibited.identity.asset_id = "backed:NVDA_prohibited".to_string();

        let prohibited_err = register_asset_at(
            nvda_prohibited,
            valid_ownership.clone(),
            clean_financials.clone(),
            BusinessCategory::Alcohol,
            &policy,
            now,
        );
        assert!(matches!(
            prohibited_err,
            Err(RegistryError::ShariahRejected {
                reason: ShariahRejectionReason::ProhibitedBusiness,
                ..
            })
        ));

        // 4. Excessive debt rejected
        let mut excess_debt_financials = clean_financials.clone();
        excess_debt_financials.debt_ratio_bps = 3_500; // 35% > 30%
        let mut nvda_debt = nvda_asset.clone();
        nvda_debt.identity.asset_id = "backed:NVDA_debt".to_string();

        let debt_err = register_asset_at(
            nvda_debt,
            valid_ownership.clone(),
            excess_debt_financials,
            BusinessCategory::Technology,
            &policy,
            now,
        );
        assert!(matches!(
            debt_err,
            Err(RegistryError::ShariahRejected {
                reason: ShariahRejectionReason::ExcessDebt,
                ..
            })
        ));

        // 5. Expired review rejected
        let expired_now = 1_715_000_000; // Past 1_710_000_000
        let mut nvda_expired = nvda_asset.clone();
        nvda_expired.identity.asset_id = "backed:NVDA_expired".to_string();

        let expired_err = register_asset_at(
            nvda_expired,
            valid_ownership.clone(),
            clean_financials.clone(),
            BusinessCategory::Technology,
            &policy,
            expired_now,
        );
        assert!(matches!(expired_err, Err(RegistryError::ReviewExpired(_))));

        // When registered asset reaches expiration time, is_tradeable becomes false
        assert!(!registry.is_tradeable("NVDA", expired_now));

        // 6. Revoked asset is not tradeable
        assert!(registry.revoke_asset("backed:NVDAx").is_ok());
        assert!(!registry.is_tradeable("NVDA", now));
        assert_eq!(
            registry.get_eligibility("backed:NVDAx").unwrap().status,
            ShariahStatus::Revoked
        );
    }

    #[test]
    fn test_threshold_comparison_semantics_strict_vs_inclusive() {
        let policy_inclusive = ScreeningPolicy::board_approved_v1(); // LessThanOrEqual, debt limit 3,000 bps
        assert_eq!(
            policy_inclusive.comparison,
            ThresholdComparison::LessThanOrEqual
        );

        let policy_strict = ScreeningPolicy::aaoifi_21("2024"); // StrictLessThan, debt limit 3,000 bps
        assert_eq!(
            policy_strict.comparison,
            ThresholdComparison::StrictLessThan
        );

        // Case 1: Exact boundary (3,000 bps)
        let boundary_metrics = ShariahFinancialMetrics {
            debt_ratio_bps: 3_000,
            interest_bearing_cash_ratio_bps: 2_000,
            impure_income_ratio_bps: 100,
        };

        // Under LessThanOrEqual: 3,000 <= 3,000 is COMPLIANT (Pass)
        assert!(screen_financial_metrics(&boundary_metrics, &policy_inclusive).is_ok());

        // Under StrictLessThan: 3,000 < 3,000 is NOT compliant (Fail -> ExcessDebt)
        assert_eq!(
            screen_financial_metrics(&boundary_metrics, &policy_strict),
            Err(ShariahRejectionReason::ExcessDebt)
        );

        // Case 2: Just below boundary (2,999 bps)
        let below_boundary = ShariahFinancialMetrics {
            debt_ratio_bps: 2_999,
            interest_bearing_cash_ratio_bps: 2_000,
            impure_income_ratio_bps: 100,
        };

        // Both LessThanOrEqual and StrictLessThan pass for 2,999 bps
        assert!(screen_financial_metrics(&below_boundary, &policy_inclusive).is_ok());
        assert!(screen_financial_metrics(&below_boundary, &policy_strict).is_ok());

        // Case 3: Just above boundary (3,001 bps)
        let above_boundary = ShariahFinancialMetrics {
            debt_ratio_bps: 3_001,
            interest_bearing_cash_ratio_bps: 2_000,
            impure_income_ratio_bps: 100,
        };

        // Both fail for 3,001 bps
        assert_eq!(
            screen_financial_metrics(&above_boundary, &policy_inclusive),
            Err(ShariahRejectionReason::ExcessDebt)
        );
        assert_eq!(
            screen_financial_metrics(&above_boundary, &policy_strict),
            Err(ShariahRejectionReason::ExcessDebt)
        );
    }
}
