//! Shared deterministic domain logic for Equity Catalyst.

pub mod agent;
pub mod allocation;
pub mod asset;
pub mod constants;
pub mod liquidity;
pub mod math;
pub mod policy;
pub mod portfolio;
pub mod provider;
pub mod risk;
pub mod types;
pub mod validation;

pub use agent::*;
pub use allocation::*;
pub use asset::*;
pub use constants::*;
pub use liquidity::*;
pub use math::*;
pub use policy::*;
pub use portfolio::*;
pub use provider::*;
pub use risk::*;
pub use types::*;
pub use validation::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basis_points_math() {
        let bp = BasisPoints::new(2_500).unwrap();
        assert_eq!(bp.as_bps(), 2_500);
        assert_eq!(bp.to_percentage_f64(), 25.0);

        // Exceeding 10,000 should fail
        assert!(BasisPoints::new(10_001).is_err());

        // Compute bps amount
        let amount: u64 = 1_000_000;
        let portion = compute_bps_amount(amount, bp).unwrap();
        assert_eq!(portion, 250_000);
    }

    #[test]
    fn test_allocation_weights_validation() {
        let valid_weights = vec![
            AssetWeight {
                symbol: "NVDA".to_string(),
                target_weight: BasisPoints(4_000), // 40%
            },
            AssetWeight {
                symbol: "AAPL".to_string(),
                target_weight: BasisPoints(3_500), // 35%
            },
            AssetWeight {
                symbol: "USDC".to_string(),
                target_weight: BasisPoints(2_500), // 25%
            },
        ];

        let target = AllocationTarget::new(valid_weights);
        assert!(target.is_ok());
        assert_eq!(target.unwrap().total_weight_bps(), 10_000);

        // Invalid weights summing to 9,000 bps
        let invalid_weights = vec![
            AssetWeight {
                symbol: "NVDA".to_string(),
                target_weight: BasisPoints(4_000),
            },
            AssetWeight {
                symbol: "AAPL".to_string(),
                target_weight: BasisPoints(5_000),
            },
        ];
        assert!(AllocationTarget::new(invalid_weights).is_err());

        // Duplicate symbol rejection
        let duplicate_weights = vec![
            AssetWeight {
                symbol: "NVDA".to_string(),
                target_weight: BasisPoints(5_000),
            },
            AssetWeight {
                symbol: "NVDA".to_string(),
                target_weight: BasisPoints(5_000),
            },
        ];
        assert!(AllocationTarget::new(duplicate_weights).is_err());
    }

    #[test]
    fn test_ltv_and_risk_checks() {
        // Loan 50k on 100k collateral = 5,000 bps (50%)
        let ltv = calculate_ltv(50_000, 100_000).unwrap();
        assert_eq!(ltv.as_bps(), 5_000);

        // Max limit 75% -> passes
        assert!(check_ltv_limit(ltv, BasisPoints(7_500)).is_ok());

        // Max limit 40% -> fails
        assert!(check_ltv_limit(ltv, BasisPoints(4_000)).is_err());

        // Zero collateral with non-zero loan gives max LTV
        assert_eq!(calculate_ltv(100, 0).unwrap(), BasisPoints::MAX);
    }

    #[test]
    fn test_exposure_limit() {
        let total_portfolio = 1_000_000;
        let position = 300_000; // 30%

        // Limit 25% -> exceeds
        assert!(check_position_exposure(position, total_portfolio, BasisPoints(2_500)).is_err());

        // Limit 35% -> passes
        assert!(check_position_exposure(position, total_portfolio, BasisPoints(3_500)).is_ok());
    }

    #[test]
    fn test_stop_loss_and_take_profit() {
        let entry_price = 100;

        // 6% drop from 100 to 94 -> triggers 5% stop loss
        assert!(is_stop_loss_triggered(entry_price, 94, BasisPoints(500)));

        // 3% drop from 100 to 97 -> does not trigger 5% stop loss
        assert!(!is_stop_loss_triggered(entry_price, 97, BasisPoints(500)));

        // 16% rise from 100 to 116 -> triggers 15% take profit
        assert!(is_take_profit_triggered(
            entry_price,
            116,
            BasisPoints(1_500)
        ));

        // 10% rise from 100 to 110 -> does not trigger 15% take profit
        assert!(!is_take_profit_triggered(
            entry_price,
            110,
            BasisPoints(1_500)
        ));
    }

    #[test]
    fn test_portfolio_rebalancing_plan() {
        let target = AllocationTarget::new(vec![
            AssetWeight {
                symbol: "NVDA".to_string(),
                target_weight: BasisPoints(5_000), // 50% = 500k
            },
            AssetWeight {
                symbol: "USDC".to_string(),
                target_weight: BasisPoints(5_000), // 50% = 500k
            },
        ])
        .unwrap();

        let positions = vec![
            PositionSnapshot {
                symbol: "NVDA".to_string(),
                current_amount: 7_000,
                entry_price_usd: 100,
                current_price_usd: 100,
                current_value_usd: 700_000, // currently 70%, +20% drift
            },
            PositionSnapshot {
                symbol: "USDC".to_string(),
                current_amount: 300_000,
                entry_price_usd: 1,
                current_price_usd: 1,
                current_value_usd: 300_000, // currently 30%, -20% drift
            },
        ];

        let total_portfolio = 1_000_000;
        let threshold = BasisPoints(200); // 2%

        assert!(is_rebalance_needed(
            &positions,
            &target,
            total_portfolio,
            threshold
        ));

        let plan =
            calculate_rebalance_plan(&positions, &target, total_portfolio, threshold).unwrap();
        assert_eq!(plan.len(), 2);

        let nvda_trade = plan.iter().find(|t| t.symbol == "NVDA").unwrap();
        assert!(!nvda_trade.is_buy); // Sell NVDA
        assert_eq!(nvda_trade.trade_value, 200_000);

        let usdc_trade = plan.iter().find(|t| t.symbol == "USDC").unwrap();
        assert!(usdc_trade.is_buy); // Buy USDC
        assert_eq!(usdc_trade.trade_value, 200_000);
    }

    #[test]
    fn test_policy_event_evaluation() {
        let policy = PolicyDefinition::default();

        let beat_signal = evaluate_event_signal("earnings_beat", 0.8, &policy);
        assert_eq!(beat_signal, Some(SignalType::Bullish));

        let miss_signal = evaluate_event_signal("earnings_miss", -0.9, &policy);
        assert_eq!(miss_signal, Some(SignalType::Bearish));

        let emergency_signal = evaluate_event_signal("circuit_breaker", 0.0, &policy);
        assert_eq!(emergency_signal, Some(SignalType::EmergencyExit));

        // Inactive policy gives None
        let mut inactive_policy = policy.clone();
        inactive_policy.is_active = false;
        assert_eq!(
            evaluate_event_signal("earnings_beat", 0.8, &inactive_policy),
            None
        );
    }

    #[test]
    fn test_math_comprehensive() {
        // compute_bps_amount
        assert_eq!(compute_bps_amount(100_000, BasisPoints(0)).unwrap(), 0);
        assert_eq!(
            compute_bps_amount(100_000, BasisPoints(5_000)).unwrap(),
            50_000
        );
        assert_eq!(
            compute_bps_amount(100_000, BasisPoints(10_000)).unwrap(),
            100_000
        );
        assert!(compute_bps_amount(100_000, BasisPoints(10_001)).is_err());

        // calculate_basis_points
        assert_eq!(calculate_basis_points(0, 1_000).unwrap(), BasisPoints(0));
        assert_eq!(
            calculate_basis_points(250, 1_000).unwrap(),
            BasisPoints(2_500)
        );
        assert_eq!(
            calculate_basis_points(1_000, 1_000).unwrap(),
            BasisPoints(10_000)
        );
        assert_eq!(calculate_basis_points(50, 0).unwrap(), BasisPoints(0)); // Zero total safe
        assert_eq!(
            calculate_basis_points(2_000, 1_000).unwrap(),
            BasisPoints(10_000)
        ); // Capped at MAX

        // calculate_drift
        assert_eq!(
            calculate_drift(BasisPoints(2_000), BasisPoints(2_000)),
            BasisPoints(0)
        );
        assert_eq!(
            calculate_drift(BasisPoints(3_500), BasisPoints(2_000)),
            BasisPoints(1_500)
        );
        assert_eq!(
            calculate_drift(BasisPoints(1_000), BasisPoints(2_500)),
            BasisPoints(1_500)
        );

        // calculate_shares_to_mint
        // Initial deposit 1:1
        assert_eq!(calculate_shares_to_mint(100_000, 0, 0).unwrap(), 100_000);
        // Subsequent deposit when total_deposits == total_shares (share price = 1.0)
        assert_eq!(
            calculate_shares_to_mint(50_000, 100_000, 100_000).unwrap(),
            50_000
        );
        // Share price = 1.25 (total_deposits = 125k, total_shares = 100k) -> 25k deposit yields 20k shares
        assert_eq!(
            calculate_shares_to_mint(25_000, 125_000, 100_000).unwrap(),
            20_000
        );

        // calculate_assets_to_withdraw
        assert_eq!(
            calculate_assets_to_withdraw(0, 100_000, 100_000).unwrap(),
            0
        );
        assert_eq!(
            calculate_assets_to_withdraw(20_000, 125_000, 100_000).unwrap(),
            25_000
        );
        assert_eq!(
            calculate_assets_to_withdraw(100_000, 125_000, 100_000).unwrap(),
            125_000
        );

        // calculate_slippage_bps
        assert_eq!(
            calculate_slippage_bps(1_000, 1_000).unwrap(),
            BasisPoints(0)
        );
        assert_eq!(
            calculate_slippage_bps(1_000, 1_050).unwrap(),
            BasisPoints(0)
        ); // Favorable
        assert_eq!(
            calculate_slippage_bps(1_000, 950).unwrap(),
            BasisPoints(500)
        ); // 5% slippage = 500 bps
    }

    #[test]
    fn test_allocation_comprehensive() {
        // Empty weights rejection
        assert_eq!(
            AllocationTarget::new(vec![]),
            Err(ValidationError::EmptyAllocationWeights)
        );

        // Sum under 10,000 bps
        let under_weights = vec![AssetWeight {
            symbol: "SOL".to_string(),
            target_weight: BasisPoints(9_999),
        }];
        assert_eq!(
            AllocationTarget::new(under_weights),
            Err(ValidationError::WeightsDoNotSumTo100Percent {
                actual: 9_999,
                expected: 10_000
            })
        );

        // Sum over 10,000 bps
        let over_weights = vec![
            AssetWeight {
                symbol: "SOL".to_string(),
                target_weight: BasisPoints(6_000),
            },
            AssetWeight {
                symbol: "USDC".to_string(),
                target_weight: BasisPoints(4_001),
            },
        ];
        assert_eq!(
            AllocationTarget::new(over_weights),
            Err(ValidationError::WeightsDoNotSumTo100Percent {
                actual: 10_001,
                expected: 10_000
            })
        );

        // Rebalance plan within drift threshold produces no trades
        let balanced_target = AllocationTarget::new(vec![
            AssetWeight {
                symbol: "SOL".to_string(),
                target_weight: BasisPoints(5_000),
            },
            AssetWeight {
                symbol: "USDC".to_string(),
                target_weight: BasisPoints(5_000),
            },
        ])
        .unwrap();

        let aligned_positions = vec![
            PositionSnapshot {
                symbol: "SOL".to_string(),
                current_amount: 5_000,
                entry_price_usd: 100,
                current_price_usd: 100,
                current_value_usd: 505_000, // 50.5% (50 bps drift)
            },
            PositionSnapshot {
                symbol: "USDC".to_string(),
                current_amount: 495_000,
                entry_price_usd: 1,
                current_price_usd: 1,
                current_value_usd: 495_000, // 49.5% (50 bps drift)
            },
        ];

        // 100 bps threshold ignores 50 bps drift
        let threshold = BasisPoints(100);
        let plan =
            calculate_rebalance_plan(&aligned_positions, &balanced_target, 1_000_000, threshold)
                .unwrap();
        assert_eq!(plan.len(), 0);
    }

    #[test]
    fn test_policy_comprehensive() {
        assert!(is_bullish_event("earnings_beat"));
        assert!(is_bullish_event("product_launch"));
        assert!(!is_bullish_event("regular_transfer"));

        assert!(is_bearish_event("earnings_miss"));
        assert!(is_bearish_event("regulatory_action"));
        assert!(!is_bearish_event("earnings_beat"));

        assert!(is_emergency_event("circuit_breaker"));
        assert!(is_emergency_event("exploit_detected"));
        assert!(!is_emergency_event("earnings_beat"));

        let policy = PolicyDefinition::default();
        let target = AllocationTarget::new(vec![
            AssetWeight {
                symbol: "NVDA".to_string(),
                target_weight: BasisPoints(3_000),
            },
            AssetWeight {
                symbol: "USDC".to_string(),
                target_weight: BasisPoints(7_000),
            },
        ])
        .unwrap();

        // Bullish event increases target asset weight by delta (500 bps)
        let adjusted_target =
            evaluate_policy_event(&target, "earnings_beat", "NVDA", 0.85, &policy).unwrap();
        assert_eq!(adjusted_target.get_weight("NVDA").unwrap().0, 3_500);
        assert_eq!(adjusted_target.get_weight("USDC").unwrap().0, 6_500);

        // Emergency event liquidates risk assets to 100% USDC
        let emergency_target =
            evaluate_policy_event(&target, "circuit_breaker", "NVDA", 0.0, &policy).unwrap();
        assert_eq!(emergency_target.get_weight("USDC").unwrap().0, 10_000);
    }

    #[test]
    fn test_risk_comprehensive() {
        // calculate_ltv
        assert_eq!(calculate_ltv(0, 100_000).unwrap(), BasisPoints(0));
        assert_eq!(calculate_ltv(65_000, 100_000).unwrap(), BasisPoints(6_500));
        assert_eq!(calculate_ltv(100, 0).unwrap(), BasisPoints(10_000)); // Zero collateral

        // check_ltv_limit
        assert!(check_ltv_limit(BasisPoints(6_500), BasisPoints(6_500)).is_ok());
        assert!(check_ltv_limit(BasisPoints(6_501), BasisPoints(6_500)).is_err());

        // check_position_exposure
        assert!(check_position_exposure(250_000, 1_000_000, BasisPoints(2_500)).is_ok());
        assert!(check_position_exposure(250_100, 1_000_000, BasisPoints(2_500)).is_err());

        // is_stop_loss_triggered
        assert!(is_stop_loss_triggered(100, 91, BasisPoints(800))); // 9% drop > 8% stop
        assert!(!is_stop_loss_triggered(100, 93, BasisPoints(800))); // 7% drop < 8% stop

        // is_take_profit_triggered
        assert!(is_take_profit_triggered(100, 121, BasisPoints(2_000))); // 21% gain > 20% target
        assert!(!is_take_profit_triggered(100, 119, BasisPoints(2_000))); // 19% gain < 20% target
    }

    #[test]
    fn test_validation_comprehensive() {
        // validate_bps
        assert_eq!(validate_bps(0).unwrap(), BasisPoints(0));
        assert_eq!(validate_bps(10_000).unwrap(), BasisPoints(10_000));
        assert!(validate_bps(10_001).is_err());

        // validate_mint_address
        let valid_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        assert!(validate_mint_address(valid_mint).is_ok());
        assert!(validate_mint_address("So11111111111111111111111111111111111111112").is_ok());

        // Invalid: too short
        assert!(validate_mint_address("EPjFWdd5Aufq").is_err());
        // Invalid: non-base58 char '0', 'O', 'I', 'l'
        assert!(validate_mint_address("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt00").is_err());

        // validate_name_and_symbol
        assert!(validate_name_and_symbol("Liquid Growth Vault", "LGV").is_ok());
        assert!(validate_name_and_symbol("", "LGV").is_err()); // Empty name
        assert!(validate_name_and_symbol("Liquid Growth Vault", "").is_err()); // Empty symbol
        assert!(validate_name_and_symbol(
            "This is an extraordinarily long vault name that exceeds thirty two characters",
            "LGV"
        )
        .is_err());
        assert!(validate_name_and_symbol("Liquid Growth Vault", "TOOLONGSYMBOL123").is_err());
    }
}
