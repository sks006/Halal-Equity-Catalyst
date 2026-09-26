//! Integration test suite for Phase 10: Deterministic Policy & Risk Authorization Layer.
//!
//! # Core Security Invariant Tested:
//! "AI may propose. AI may NOT authorize execution, bypass risk controls,
//! bypass Shariah rules, choose arbitrary signing keys, or directly submit transactions."
//!
//! Evaluates all 13 deterministic criteria:
//! 1. Asset active
//! 2. Asset Shariah-approved
//! 3. Asset ownership valid
//! 4. Portfolio limits
//! 5. Position limits
//! 6. Maximum trade size
//! 7. Available balance (asset and cash)
//! 8. Solvency (cash reserve retention without leverage)
//! 9. Oracle freshness
//! 10. Oracle confidence
//! 11. DEX quote validity
//! 12. Slippage
//! 13. Price impact

use chrono::{DateTime, TimeZone, Utc};
use equity_catalyst_api::{
    models::{CanonicalAssetModel, PolicyModel, PortfolioModel},
    AllocationProposal, DeterministicPolicyAuthorizer, MarketPriceUpdate, PolicyRejectionReason,
    ValidationContext,
};
use equity_catalyst_jupiter::{DexQuote, DexRouteInfo, DexRouteStep};
use equity_catalyst_shared::{
    asset::{
        Asset, AssetIdentity, AssetProvider, AssetStatus, AssetType, Network, ProviderConfig,
        TokenDetails,
    },
    shariah::{
        DenominatorMethod, RegisteredAsset, ScreeningStandard, ShariahEligibility, ShariahStatus,
    },
    AssetApprovalStatus,
};
use uuid::Uuid;

// Constant addresses and mints for tests
const VAULT_ADDR: &str = "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4";
const AAPL_MINT: &str = "AAPL111111111111111111111111111111111111111";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const FEED_AAPL: &str = "feed1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

/// Test fixture generator producing a baseline compliant setup.
struct TestFixture {
    policy: PolicyModel,
    positions: Vec<PortfolioModel>,
    total_portfolio_usd: u64,
    available_cash_usd: u64,
    canonical_asset: CanonicalAssetModel,
    shariah_record: RegisteredAsset,
    oracle_price: MarketPriceUpdate,
    dex_quote: DexQuote,
    evaluation_time: DateTime<Utc>,
    proposal: AllocationProposal,
}

impl TestFixture {
    fn new_baseline() -> Self {
        let now = Utc.timestamp_opt(1_750_000_000, 0).unwrap();

        let policy = PolicyModel {
            policy_address: "pol-baseline-001".to_string(),
            vault_address: VAULT_ADDR.to_string(),
            authority: "auth-baseline".to_string(),
            min_cash_bps: 1000,     // 10.00% minimum cash reserve
            max_position_bps: 2500, // 25.00% max single position
            stop_loss_bps: 800,
            take_profit_bps: 2000,
            rebalance_threshold_bps: 150,
            is_active: true,
            bump: 255,
            created_at: now,
            updated_at: now,
        };

        // Existing position: AAPL at 10% weight ($10,000)
        let positions = vec![PortfolioModel {
            portfolio_id: Uuid::new_v4(),
            vault_address: VAULT_ADDR.to_string(),
            asset_symbol: "AAPL".to_string(),
            asset_mint: AAPL_MINT.to_string(),
            amount: 50_000_000, // 50 shares
            entry_price_usd: 190.0,
            current_price_usd: 200.0,
            current_value_usd: 10_000.0,
            target_weight_bps: 1500,
            current_weight_bps: 1000,
            last_rebalanced_at: Some(now),
            updated_at: now,
        }];

        let total_portfolio_usd = 100_000; // $100,000 total portfolio
        let available_cash_usd = 40_000; // $40,000 cash (40%)

        let canonical_asset = CanonicalAssetModel {
            asset_id: "backed:AAPL".to_string(),
            symbol: "AAPL".to_string(),
            mint_address: AAPL_MINT.to_string(),
            legal_issuer: "Backed Finance AG".to_string(),
            custodian: "Incore Bank AG".to_string(),
            underlying_asset_identifier: "US0378331005".to_string(),
            is_active: true,
            approval_status: AssetApprovalStatus::ShariahApproved,
            decimals: 6,
            created_at: now,
            updated_at: now,
        };

        let asset = Asset {
            identity: AssetIdentity {
                asset_id: "backed:AAPL".to_string(),
                symbol: "AAPL".to_string(),
                name: "Apple Inc.".to_string(),
                asset_type: AssetType::Stock,
                underlying_reference: "US0378331005".to_string(),
            },
            token: TokenDetails {
                mint: AAPL_MINT.to_string(),
                decimals: 6,
                network: Network::SolanaMainnet,
            },
            provider: ProviderConfig {
                provider: AssetProvider::Backed,
                price_feed_id: FEED_AAPL.to_string(),
                meteora_pool: None,
                secondary_reference: None,
                status: AssetStatus::Active,
            },
        };

        let eligibility = ShariahEligibility {
            status: ShariahStatus::Approved,
            standard: ScreeningStandard::Aaoifi21,
            business_activity_approved: true,
            debt_ratio_bps: 1250,           // 12.50% (< 33%)
            interest_bearing_cash_bps: 800, // 8.00% (< 33%)
            receivables_cash_bps: Some(400),
            impure_income_bps: 50, // 0.50% (< 5%)
            denominator_method: DenominatorMethod::CurrentMarketCap,
            ownership_verified: true,
            evidence_hash: "d4b8e2f1837bc21".to_string(),
            reviewed_at: now.timestamp() - 3600,
            expires_at: now.timestamp() + 86400 * 30, // 30 days remaining
            policy_version: "v1.0".to_string(),
        };

        let shariah_record = RegisteredAsset { asset, eligibility };

        let mut oracle_price = MarketPriceUpdate::from_raw(
            "backed:AAPL",
            "AAPL",
            AAPL_MINT,
            FEED_AAPL,
            200_00000000, // $200.00
            -8,
            2000000,             // conf = 0.02 (10 bps)
            now.timestamp() - 5, // 5s ago
            30,
            Some(12345),
        )
        .expect("Valid oracle price");
        oracle_price.is_stale = false;

        let dex_quote = DexQuote {
            quote_id: "quote-test-001".to_string(),
            provider_id: "jupiter".to_string(),
            input_mint: USDC_MINT.to_string(),
            output_mint: AAPL_MINT.to_string(),
            input_amount: 5_000_000_000,        // 5,000 USDC
            expected_output_amount: 25_000_000, // 25 AAPL
            minimum_output_amount: 24_875_000,  // with 50 bps slippage
            price_impact_bps: 12,               // 0.12%
            price_impact_pct: "0.12%".to_string(),
            effective_rate: 0.005,
            quote_timestamp: now.timestamp() - 2,
            expires_at: now.timestamp() + 45, // expires in 45s
            ttl_seconds: 60,
            route_info: DexRouteInfo {
                steps: vec![DexRouteStep {
                    dex_label: "Orca Whirlpool".to_string(),
                    amm_key: "WhirlpoolAAPL".to_string(),
                    input_mint: USDC_MINT.to_string(),
                    output_mint: AAPL_MINT.to_string(),
                    in_amount: 5_000_000_000,
                    out_amount: 25_000_000,
                    percent: 100,
                }],
                num_hops: 1,
                primary_dex: "Orca Whirlpool".to_string(),
            },
            raw_payload: None,
        };

        // Proposal: BUY $5,000 worth of AAPL (takes position from $10k to $15k = 15% of $100k, well within 25% max)
        let proposal = AllocationProposal {
            proposal_id: Uuid::new_v4(),
            vault_address: VAULT_ADDR.to_string(),
            asset_id: "backed:AAPL".to_string(),
            symbol: "AAPL".to_string(),
            mint_address: AAPL_MINT.to_string(),
            is_buy: true,
            target_amount: 25_000_000,
            proposed_usd_value: 5_000,
            max_slippage_bps: 50,
            proposed_at: now,
            ai_confidence: Some(0.98),
            ai_rationale: Some("Mean-reversion rebalance target".to_string()),
            ai_approved: false,
        };

        Self {
            policy,
            positions,
            total_portfolio_usd,
            available_cash_usd,
            canonical_asset,
            shariah_record,
            oracle_price,
            dex_quote,
            evaluation_time: now,
            proposal,
        }
    }

    fn to_context(&self) -> ValidationContext<'_> {
        ValidationContext {
            policy: &self.policy,
            positions: &self.positions,
            total_portfolio_usd: self.total_portfolio_usd,
            available_cash_usd: self.available_cash_usd,
            canonical_asset: Some(&self.canonical_asset),
            shariah_record: Some(&self.shariah_record),
            shariah_registry: None,
            oracle_price: Some(&self.oracle_price),
            dex_quote: Some(&self.dex_quote),
            evaluation_time: self.evaluation_time,
            max_trade_size_usd: None,
            max_oracle_conf_bps: None,
            max_allowed_slippage_bps: None,
            max_allowed_price_impact_bps: None,
            max_oracle_staleness_secs: None,
        }
    }
}

#[test]
fn test_rule_1_asset_inactive_rejected() {
    let mut fixture = TestFixture::new_baseline();
    fixture.canonical_asset.is_active = false;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(
        !outcome.is_authorized(),
        "Inactive asset must NOT be authorized"
    );
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::AssetInactive { .. })),
        "Expected AssetInactive rejection reason, got: {:?}",
        reasons
    );
}

#[test]
fn test_rule_2_shariah_non_compliant_rejected() {
    let authorizer = DeterministicPolicyAuthorizer::new();

    // Case A: Status is Rejected
    {
        let mut fixture = TestFixture::new_baseline();
        fixture.shariah_record.eligibility.status = ShariahStatus::Rejected;
        let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());
        assert!(!outcome.is_authorized());
        let reasons = outcome.rejection_reasons().unwrap();
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::ShariahNonCompliant { .. })));
    }

    // Case B: Business activity not permissible
    {
        let mut fixture = TestFixture::new_baseline();
        fixture
            .shariah_record
            .eligibility
            .business_activity_approved = false;
        let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());
        assert!(!outcome.is_authorized());
        let reasons = outcome.rejection_reasons().unwrap();
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::ShariahNonCompliant { .. })));
    }

    // Case C: Review has expired
    {
        let mut fixture = TestFixture::new_baseline();
        fixture.shariah_record.eligibility.expires_at = fixture.evaluation_time.timestamp() - 100;
        let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());
        assert!(!outcome.is_authorized());
        let reasons = outcome.rejection_reasons().unwrap();
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::ShariahNonCompliant { .. })));
    }

    // Case D: Missing from registry entirely
    {
        let fixture = TestFixture::new_baseline();
        let mut ctx = fixture.to_context();
        ctx.shariah_record = None;
        ctx.shariah_registry = None;
        let outcome = authorizer.authorize(&fixture.proposal, &ctx);
        assert!(!outcome.is_authorized());
        let reasons = outcome.rejection_reasons().unwrap();
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::ShariahNonCompliant { .. })));
    }
}

#[test]
fn test_rule_3_asset_ownership_unverified_rejected() {
    let mut fixture = TestFixture::new_baseline();
    fixture.shariah_record.eligibility.ownership_verified = false;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(!outcome.is_authorized());
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::AssetOwnershipUnverified { .. })),
        "Expected AssetOwnershipUnverified, got: {:?}",
        reasons
    );
}

#[test]
fn test_rule_4_portfolio_limits_zero_valuation_rejected() {
    let mut fixture = TestFixture::new_baseline();
    fixture.total_portfolio_usd = 0;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(!outcome.is_authorized());
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::PortfolioLimitExceeded { .. })),
        "Expected PortfolioLimitExceeded, got: {:?}",
        reasons
    );
}

#[test]
fn test_rule_5_position_limit_exceeded_rejected() {
    let mut fixture = TestFixture::new_baseline();
    // Max position is 25% = $25,000. Current is $10,000.
    // Proposing $20,000 brings total to $30,000 (30%), which breaches the 25% limit.
    fixture.proposal.proposed_usd_value = 20_000;
    fixture.proposal.target_amount = 100_000_000;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(!outcome.is_authorized());
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::PositionLimitExceeded { .. })),
        "Expected PositionLimitExceeded, got: {:?}",
        reasons
    );
}

#[test]
fn test_rule_6_max_trade_size_exceeded_rejected() {
    let fixture = TestFixture::new_baseline();
    // Set custom max trade size of $3,000. Proposal is $5,000.
    let mut ctx = fixture.to_context();
    ctx.max_trade_size_usd = Some(3_000);

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &ctx);

    assert!(!outcome.is_authorized());
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::MaxTradeSizeExceeded { .. })),
        "Expected MaxTradeSizeExceeded, got: {:?}",
        reasons
    );
}

#[test]
fn test_rule_7_available_balance_sell_insufficient_rejected() {
    let mut fixture = TestFixture::new_baseline();
    // Switch to a SELL trade
    fixture.proposal.is_buy = false;
    fixture.dex_quote.input_mint = AAPL_MINT.to_string();
    fixture.dex_quote.output_mint = USDC_MINT.to_string();

    // Vault holds 50 shares (50_000_000). Propose selling 100 shares (100_000_000).
    fixture.proposal.target_amount = 100_000_000;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(
        !outcome.is_authorized(),
        "Overselling unowned assets (Bay' ma la Yamlik) must be rejected"
    );
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::InsufficientAssetBalance { .. })),
        "Expected InsufficientAssetBalance, got: {:?}",
        reasons
    );
}

#[test]
fn test_rule_8_available_balance_buy_insufficient_cash_rejected() {
    let mut fixture = TestFixture::new_baseline();
    // Cash available is only $2,000. Proposed buy is $5,000.
    fixture.available_cash_usd = 2_000;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(!outcome.is_authorized());
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::InsufficientCashBalance { .. })),
        "Expected InsufficientCashBalance, got: {:?}",
        reasons
    );
}

#[test]
fn test_rule_9_solvency_cash_reserve_breach_rejected() {
    let mut fixture = TestFixture::new_baseline();
    // Total portfolio = $100,000. Min cash = 10% = $10,000 reserve.
    // Available cash is $12,000. Proposed buy is $5,000.
    // Remaining cash would be $7,000 (< $10,000 reserve), breaching solvency.
    fixture.available_cash_usd = 12_000;
    fixture.proposal.proposed_usd_value = 5_000;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(!outcome.is_authorized());
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::SolvencyBreach { .. })),
        "Expected SolvencyBreach, got: {:?}",
        reasons
    );
}

#[test]
fn test_rule_10_oracle_freshness_rejected() {
    let authorizer = DeterministicPolicyAuthorizer::new();

    // Case A: Oracle is stale (older than 30s)
    {
        let mut fixture = TestFixture::new_baseline();
        fixture.oracle_price.publish_time = fixture.evaluation_time.timestamp() - 65; // 65s ago
        let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());
        assert!(!outcome.is_authorized());
        let reasons = outcome.rejection_reasons().unwrap();
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::OraclePriceStale { .. })));
    }

    // Case B: Oracle is future-skewed by > 5s
    {
        let mut fixture = TestFixture::new_baseline();
        fixture.oracle_price.publish_time = fixture.evaluation_time.timestamp() + 10;
        let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());
        assert!(!outcome.is_authorized());
        let reasons = outcome.rejection_reasons().unwrap();
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::OraclePriceStale { .. })));
    }
}

#[test]
fn test_rule_11_oracle_confidence_too_wide_rejected() {
    let mut fixture = TestFixture::new_baseline();
    // Default max conf is 50 bps (0.50%). Set confidence interval to 80 bps.
    fixture.oracle_price.conf_bps = 80;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(!outcome.is_authorized());
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::OracleConfidenceTooWide { .. })),
        "Expected OracleConfidenceTooWide, got: {:?}",
        reasons
    );
}

#[test]
fn test_rule_12_dex_quote_invalid_or_expired_rejected() {
    let authorizer = DeterministicPolicyAuthorizer::new();

    // Case A: Missing quote
    {
        let fixture = TestFixture::new_baseline();
        let mut ctx = fixture.to_context();
        ctx.dex_quote = None;
        let outcome = authorizer.authorize(&fixture.proposal, &ctx);
        assert!(!outcome.is_authorized());
        let reasons = outcome.rejection_reasons().unwrap();
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::DexQuoteInvalid { .. })));
    }

    // Case B: Expired quote
    {
        let mut fixture = TestFixture::new_baseline();
        fixture.dex_quote.expires_at = fixture.evaluation_time.timestamp() - 5;
        let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());
        assert!(!outcome.is_authorized());
        let reasons = outcome.rejection_reasons().unwrap();
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::DexQuoteInvalid { .. })));
    }

    // Case C: Output mint mismatch for BUY trade
    {
        let mut fixture = TestFixture::new_baseline();
        fixture.dex_quote.output_mint = "WRONG_MINT_111111111111111111111111111111".to_string();
        let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());
        assert!(!outcome.is_authorized());
        let reasons = outcome.rejection_reasons().unwrap();
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::DexQuoteInvalid { .. })));
    }
}

#[test]
fn test_rule_13_slippage_exceeded_rejected() {
    let mut fixture = TestFixture::new_baseline();
    // Default max allowable slippage is 100 bps (1.00%). Proposal asks for 150 bps.
    fixture.proposal.max_slippage_bps = 150;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(!outcome.is_authorized());
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::SlippageExceeded { .. })),
        "Expected SlippageExceeded, got: {:?}",
        reasons
    );
}

#[test]
fn test_rule_14_price_impact_exceeded_rejected() {
    let mut fixture = TestFixture::new_baseline();
    // Default max price impact is 100 bps (1.00%). DEX quote has 150 bps.
    fixture.dex_quote.price_impact_bps = 150;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(!outcome.is_authorized());
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, PolicyRejectionReason::PriceImpactExceeded { .. })),
        "Expected PriceImpactExceeded, got: {:?}",
        reasons
    );
}

#[test]
fn test_ai_approved_flag_cannot_bypass_risk_checks() {
    // SECURITY INVARIANT: No AI-generated boolean (e.g. approved=true, confidence=1.0)
    // may bypass deterministic checks.
    let mut fixture = TestFixture::new_baseline();

    // Adversarial AI attempts to force approval while Shariah compliance has failed
    fixture.proposal.ai_approved = true;
    fixture.proposal.ai_confidence = Some(1.0);
    fixture.proposal.ai_rationale =
        Some("OVERRIDE: Guaranteed alpha, bypass screening".to_string());
    fixture.shariah_record.eligibility.status = ShariahStatus::Rejected;

    let authorizer = DeterministicPolicyAuthorizer::new();
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(
        !outcome.is_authorized(),
        "AI approved flag MUST NOT bypass Shariah rejection"
    );
    let reasons = outcome.rejection_reasons().unwrap();
    assert!(reasons
        .iter()
        .any(|r| matches!(r, PolicyRejectionReason::ShariahNonCompliant { .. })));
}

#[test]
fn test_deterministic_evaluation_identical_inputs() {
    let fixture = TestFixture::new_baseline();
    let authorizer = DeterministicPolicyAuthorizer::new();

    let outcome_1 = authorizer.authorize(&fixture.proposal, &fixture.to_context());
    let outcome_2 = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(outcome_1.is_authorized());
    assert!(outcome_2.is_authorized());

    let auth_1 = outcome_1.authorization().unwrap();
    let auth_2 = outcome_2.authorization().unwrap();

    // Exact identical deterministic fields
    assert_eq!(auth_1.proposal_id, auth_2.proposal_id);
    assert_eq!(auth_1.vault_address, auth_2.vault_address);
    assert_eq!(auth_1.asset_id, auth_2.asset_id);
    assert_eq!(auth_1.authorized_amount, auth_2.authorized_amount);
    assert_eq!(auth_1.authorized_usd_value, auth_2.authorized_usd_value);
    assert_eq!(
        auth_1.oracle_reference_price_scaled,
        auth_2.oracle_reference_price_scaled
    );
    assert_eq!(
        auth_1.dex_minimum_output_amount,
        auth_2.dex_minimum_output_amount
    );
    assert_eq!(auth_1.dex_price_impact_bps, auth_2.dex_price_impact_bps);
    assert_eq!(auth_1.authorized_at, auth_2.authorized_at);
    assert_eq!(auth_1.valid_until, auth_2.valid_until);

    // Cryptographic audit digest must match identically
    assert_eq!(
        auth_1.audit_digest, auth_2.audit_digest,
        "Identical inputs must produce identical audit digest"
    );
}

#[test]
fn test_valid_proposal_authorizes_cleanly() {
    let fixture = TestFixture::new_baseline();
    let authorizer = DeterministicPolicyAuthorizer::new();

    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());

    assert!(
        outcome.is_authorized(),
        "Compliant baseline proposal must authorize"
    );
    let auth = outcome.authorization().unwrap();

    assert_eq!(auth.proposal_id, fixture.proposal.proposal_id);
    assert_eq!(auth.vault_address, fixture.proposal.vault_address);
    assert_eq!(auth.symbol, "AAPL");
    assert_eq!(auth.mint_address, AAPL_MINT);
    assert!(auth.is_buy);
    assert_eq!(auth.authorized_amount, 25_000_000);
    assert_eq!(auth.authorized_usd_value, 5_000);
    assert_eq!(auth.oracle_reference_price_scaled, 200_000_000); // $200.00 micro-USD
    assert_eq!(auth.dex_expected_output_amount, 25_000_000);
    assert_eq!(auth.dex_minimum_output_amount, 24_875_000);
    assert_eq!(auth.dex_price_impact_bps, 12);
    assert_eq!(auth.valid_until.timestamp(), fixture.dex_quote.expires_at);
    assert!(!auth.audit_digest.is_empty());
}

#[test]
fn test_phase10_acceptance_criteria_proposal_to_authorization() {
    // ACCEPTANCE CRITERIA:
    // An allocation proposal can be transformed into an authorization ONLY when every deterministic rule passes.
    let mut fixture = TestFixture::new_baseline();
    let authorizer = DeterministicPolicyAuthorizer::new();

    // 1. Initially valid -> Authorized
    let outcome = authorizer.authorize(&fixture.proposal, &fixture.to_context());
    assert!(outcome.is_authorized());

    // 2. Introduce 3 independent violations (oracle stale + asset inactive + slippage exceeded)
    fixture.oracle_price.publish_time = fixture.evaluation_time.timestamp() - 100;
    fixture.canonical_asset.is_active = false;
    fixture.proposal.max_slippage_bps = 250; // exceeds 100 bps

    let outcome_multi = authorizer.authorize(&fixture.proposal, &fixture.to_context());
    assert!(
        !outcome_multi.is_authorized(),
        "Must be rejected when rules fail"
    );
    let reasons = outcome_multi.rejection_reasons().unwrap();
    assert_eq!(
        reasons.len(),
        3,
        "Expected exactly 3 distinct structured rejection reasons"
    );
    assert!(reasons
        .iter()
        .any(|r| matches!(r, PolicyRejectionReason::OraclePriceStale { .. })));
    assert!(reasons
        .iter()
        .any(|r| matches!(r, PolicyRejectionReason::AssetInactive { .. })));
    assert!(reasons
        .iter()
        .any(|r| matches!(r, PolicyRejectionReason::SlippageExceeded { .. })));

    // 3. Fix all 3 violations -> Successfully transformed to authorization!
    fixture.oracle_price.publish_time = fixture.evaluation_time.timestamp() - 5;
    fixture.canonical_asset.is_active = true;
    fixture.proposal.max_slippage_bps = 50;

    let outcome_restored = authorizer.authorize(&fixture.proposal, &fixture.to_context());
    assert!(
        outcome_restored.is_authorized(),
        "An allocation proposal can be transformed into an authorization ONLY when every deterministic rule passes."
    );
    assert!(outcome_restored.authorization().is_some());
}
