//! Shared deterministic domain logic for Equity Catalyst.

pub mod allocation;
pub mod constants;
pub mod math;
pub mod policy;
pub mod risk;
pub mod types;
pub mod validation;

pub use allocation::*;
pub use constants::*;
pub use math::*;
pub use policy::*;
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
        assert!(is_take_profit_triggered(entry_price, 116, BasisPoints(1_500)));

        // 10% rise from 100 to 110 -> does not trigger 15% take profit
        assert!(!is_take_profit_triggered(entry_price, 110, BasisPoints(1_500)));
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

        assert!(is_rebalance_needed(&positions, &target, total_portfolio, threshold));

        let plan = calculate_rebalance_plan(&positions, &target, total_portfolio, threshold).unwrap();
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
        assert_eq!(evaluate_event_signal("earnings_beat", 0.8, &inactive_policy), None);
    }
}
