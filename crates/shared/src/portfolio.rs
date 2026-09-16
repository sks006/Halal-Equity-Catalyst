//! Deterministic Portfolio and Position Domain Models.
//!
//! Enforces financial invariants, position accounting, profit/loss calculations,
//! and risk limits independently of external networks, runtimes, or AI agents.

use serde::{Deserialize, Serialize};

use crate::allocation::{calculate_rebalance_plan, RebalanceTrade};
use crate::types::{AllocationTarget, BasisPoints, PositionSnapshot};
use crate::validation::ValidationError;

/// Portfolio-level risk boundary and execution limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioLimits {
    /// Maximum allowed single position allocation in basis points (e.g. 2,500 = 25%)
    pub max_position_bps: BasisPoints,
    /// Maximum allowed single trade order size in USD
    pub max_trade_usd: f64,
    /// Maximum combined non-cash asset exposure in basis points (e.g. 8,000 = 80%)
    pub max_portfolio_exposure_bps: BasisPoints,
    /// Maximum allowed daily portfolio turnover in basis points (e.g. 5,000 = 50%)
    pub max_daily_turnover_bps: BasisPoints,
    /// Maximum allowed trade slippage in basis points (e.g. 100 = 1.00%)
    pub max_slippage_bps: BasisPoints,
    /// Maximum allowed oracle price age in seconds before price is deemed stale
    pub max_price_staleness_secs: i64,
}

impl Default for PortfolioLimits {
    fn default() -> Self {
        Self {
            max_position_bps: BasisPoints(2_500),           // 25.00%
            max_trade_usd: 50_000.0,                        // $50,000 max order
            max_portfolio_exposure_bps: BasisPoints(8_000), // 80.00%
            max_daily_turnover_bps: BasisPoints(5_000),     // 50.00%
            max_slippage_bps: BasisPoints(100),             // 1.00%
            max_price_staleness_secs: 120,                  // 2 minutes
        }
    }
}

/// Active portfolio position with deterministic valuation and PnL metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// Asset ticker symbol, e.g. "NVDA"
    pub symbol: String,
    /// Raw token balance quantity (scaled by 10^decimals)
    pub quantity: u64,
    /// Decimal precision places (standard 6 for SPL tokens / USDC, 9 for SOL)
    pub decimals: u8,
    /// Cost basis entry price in USD
    pub entry_price_usd: f64,
    /// Current oracle mark price in USD
    pub current_price_usd: f64,
}

impl Position {
    pub fn new(
        symbol: impl Into<String>,
        quantity: u64,
        decimals: u8,
        entry_price_usd: f64,
        current_price_usd: f64,
    ) -> Result<Self, ValidationError> {
        let sym = symbol.into();
        if sym.trim().is_empty() {
            return Err(ValidationError::EmptySymbol);
        }
        if entry_price_usd <= 0.0 {
            return Err(ValidationError::InvalidPrice(format!(
                "entry_price_usd must be positive, got {}",
                entry_price_usd
            )));
        }
        if current_price_usd <= 0.0 {
            return Err(ValidationError::InvalidPrice(format!(
                "current_price_usd must be positive, got {}",
                current_price_usd
            )));
        }

        Ok(Self {
            symbol: sym,
            quantity,
            decimals,
            entry_price_usd,
            current_price_usd,
        })
    }

    /// Quantity converted to real decimal units (e.g. 1_500_000 raw with 6 decimals = 1.5 units)
    #[inline]
    pub fn units(&self) -> f64 {
        (self.quantity as f64) / 10_f64.powi(self.decimals as i32)
    }

    /// Current market value in USD: units * current_price_usd
    #[inline]
    pub fn market_value_usd(&self) -> f64 {
        self.units() * self.current_price_usd
    }

    /// Total cost basis in USD: units * entry_price_usd
    #[inline]
    pub fn cost_basis_usd(&self) -> f64 {
        self.units() * self.entry_price_usd
    }

    /// Dollar unrealized profit or loss: market_value_usd - cost_basis_usd
    #[inline]
    pub fn unrealized_pnl_usd(&self) -> f64 {
        self.market_value_usd() - self.cost_basis_usd()
    }

    /// Unrealized return in basis points: ((current_price - entry_price) / entry_price) * 10,000
    pub fn unrealized_pnl_bps(&self) -> i32 {
        if self.entry_price_usd > 0.0 {
            let return_ratio =
                (self.current_price_usd - self.entry_price_usd) / self.entry_price_usd;
            (return_ratio * 10_000.0).round() as i32
        } else {
            0
        }
    }

    /// Weight of this position relative to the total portfolio value in basis points
    pub fn allocation_bps(&self, total_portfolio_value_usd: f64) -> u16 {
        if total_portfolio_value_usd > 0.0 {
            let ratio = self.market_value_usd() / total_portfolio_value_usd;
            (ratio * 10_000.0).round().clamp(0.0, 10_000.0) as u16
        } else {
            0
        }
    }
}

/// Canonical composite Portfolio model tracking cash, positions, target allocations, and risk limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portfolio {
    /// Available liquid cash reserves in USD (e.g. USDC balance)
    pub cash_usd: f64,
    /// Active asset positions held in the portfolio
    pub positions: Vec<Position>,
    /// Configured target portfolio allocation weights
    pub target_allocations: Option<AllocationTarget>,
    /// Established risk and exposure boundaries
    pub limits: PortfolioLimits,
}

impl Portfolio {
    pub fn new(
        cash_usd: f64,
        positions: Vec<Position>,
        target_allocations: Option<AllocationTarget>,
        limits: PortfolioLimits,
    ) -> Result<Self, ValidationError> {
        if cash_usd < 0.0 {
            return Err(ValidationError::InvalidParam(format!(
                "cash_usd cannot be negative, got {}",
                cash_usd
            )));
        }

        let portfolio = Self {
            cash_usd,
            positions,
            target_allocations,
            limits,
        };

        portfolio.validate_invariants()?;
        Ok(portfolio)
    }

    /// Total portfolio valuation: cash_usd + sum(position.market_value_usd)
    pub fn total_value_usd(&self) -> f64 {
        let positions_value: f64 = self.positions.iter().map(|p| p.market_value_usd()).sum();
        self.cash_usd + positions_value
    }

    /// Total non-cash asset exposure in USD
    pub fn total_position_value_usd(&self) -> f64 {
        self.positions.iter().map(|p| p.market_value_usd()).sum()
    }

    /// Lookup a specific position by ticker symbol
    pub fn get_position(&self, symbol: &str) -> Option<&Position> {
        self.positions
            .iter()
            .find(|p| p.symbol.eq_ignore_ascii_case(symbol))
    }

    /// Market value of a specific position in USD (0.0 if not held)
    pub fn position_value_usd(&self, symbol: &str) -> f64 {
        self.get_position(symbol)
            .map(|p| p.market_value_usd())
            .unwrap_or(0.0)
    }

    /// Current weight of a specific position in basis points
    pub fn position_weight_bps(&self, symbol: &str) -> u16 {
        let total = self.total_value_usd();
        if total > 0.0 {
            let val = self.position_value_usd(symbol);
            ((val / total) * 10_000.0).round().clamp(0.0, 10_000.0) as u16
        } else {
            0
        }
    }

    /// Current cash allocation weight in basis points
    pub fn cash_weight_bps(&self) -> u16 {
        let total = self.total_value_usd();
        if total > 0.0 {
            ((self.cash_usd / total) * 10_000.0)
                .round()
                .clamp(0.0, 10_000.0) as u16
        } else {
            10_000
        }
    }

    /// Total portfolio unrealized profit/loss across all positions in USD
    pub fn total_unrealized_pnl_usd(&self) -> f64 {
        self.positions.iter().map(|p| p.unrealized_pnl_usd()).sum()
    }

    /// Validates all mathematical and business invariants on the portfolio.
    pub fn validate_invariants(&self) -> Result<(), ValidationError> {
        if self.cash_usd < 0.0 {
            return Err(ValidationError::InvalidParam(format!(
                "Negative cash balance: {}",
                self.cash_usd
            )));
        }

        let total = self.total_value_usd();
        if total > 0.0 {
            // 1. Enforce max single position exposure constraint
            for pos in &self.positions {
                let weight_bps = pos.allocation_bps(total);
                if weight_bps > self.limits.max_position_bps.0 {
                    return Err(ValidationError::ExceedsMaxPosition {
                        actual: weight_bps,
                        limit: self.limits.max_position_bps.0,
                    });
                }
            }

            // 2. Enforce total portfolio exposure limit
            let total_exposure_usd = self.total_position_value_usd();
            let total_exposure_bps = ((total_exposure_usd / total) * 10_000.0).round() as u16;
            if total_exposure_bps > self.limits.max_portfolio_exposure_bps.0 {
                return Err(ValidationError::ExceedsMaxLtv {
                    actual: total_exposure_bps,
                    limit: self.limits.max_portfolio_exposure_bps.0,
                });
            }
        }

        Ok(())
    }

    /// Evaluates current allocations against target weights and produces a deterministic rebalancing proposal.
    pub fn plan_rebalancing(
        &self,
        threshold_bps: BasisPoints,
    ) -> Result<Vec<RebalanceTrade>, ValidationError> {
        let target = match &self.target_allocations {
            Some(t) => t,
            None => return Ok(Vec::new()),
        };

        let total_val = (self.total_value_usd() * 1_000_000.0).round() as u64;
        if total_val == 0 {
            return Err(ValidationError::ZeroPortfolioValue);
        }

        let mut snapshots: Vec<PositionSnapshot> = self
            .positions
            .iter()
            .map(|p| PositionSnapshot {
                symbol: p.symbol.clone(),
                current_amount: p.quantity,
                entry_price_usd: (p.entry_price_usd * 1_000_000.0).round() as u64,
                current_price_usd: (p.current_price_usd * 1_000_000.0).round() as u64,
                current_value_usd: (p.market_value_usd() * 1_000_000.0).round() as u64,
            })
            .collect();

        // If target contains USDC and cash is held, include cash balance in rebalance evaluation
        if self.cash_usd > 0.0 && !snapshots.iter().any(|s| s.symbol == "USDC") {
            let cash_scaled = (self.cash_usd * 1_000_000.0).round() as u64;
            snapshots.push(PositionSnapshot {
                symbol: "USDC".to_string(),
                current_amount: cash_scaled,
                entry_price_usd: 1_000_000,
                current_price_usd: 1_000_000,
                current_value_usd: cash_scaled,
            });
        }

        calculate_rebalance_plan(&snapshots, target, total_val, threshold_bps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_accounting_and_pnl() {
        // Bought 10 NVDA at $100, current price $120
        let pos = Position::new(
            "NVDA", 10_000_000, // 10 tokens (6 decimals)
            6, 100.0, 120.0,
        )
        .expect("Valid position");

        assert_eq!(pos.units(), 10.0);
        assert_eq!(pos.cost_basis_usd(), 1_000.0);
        assert_eq!(pos.market_value_usd(), 1_200.0);
        assert_eq!(pos.unrealized_pnl_usd(), 200.0);
        assert_eq!(pos.unrealized_pnl_bps(), 2_000); // +20.00%
        assert_eq!(pos.allocation_bps(4_800.0), 2_500); // 1,200 / 4,800 = 25.00%
    }

    #[test]
    fn test_portfolio_valuation_and_weights() {
        let pos1 = Position::new("NVDA", 10_000_000, 6, 100.0, 100.0).unwrap(); // $1,000
        let pos2 = Position::new("AAPL", 5_000_000, 6, 200.0, 200.0).unwrap(); // $1,000

        let portfolio = Portfolio::new(
            8_000.0, // $8,000 cash -> Total $10,000
            vec![pos1, pos2],
            None,
            PortfolioLimits::default(),
        )
        .expect("Valid portfolio");

        assert_eq!(portfolio.total_value_usd(), 10_000.0);
        assert_eq!(portfolio.position_weight_bps("NVDA"), 1_000); // 10.00%
        assert_eq!(portfolio.position_weight_bps("AAPL"), 1_000); // 10.00%
        assert_eq!(portfolio.cash_weight_bps(), 8_000); // 80.00%
    }

    #[test]
    fn test_portfolio_limits_violation_rejection() {
        // NVDA worth $3,000 out of $10,000 total = 30.00% > max limit 25.00%
        let pos = Position::new("NVDA", 30_000_000, 6, 100.0, 100.0).unwrap(); // $3,000

        let limits = PortfolioLimits {
            max_position_bps: BasisPoints(2_500), // 25% max
            ..Default::default()
        };

        let result = Portfolio::new(7_000.0, vec![pos], None, limits);
        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::ExceedsMaxPosition { actual, limit } => {
                assert_eq!(actual, 3_000);
                assert_eq!(limit, 2_500);
            }
            other => panic!("Expected ExceedsMaxPosition, got {:?}", other),
        }
    }

    #[test]
    fn test_portfolio_plan_rebalancing() {
        use crate::types::AssetWeight;

        // Portfolio: $5,000 cash, $5,000 NVDA (50% NVDA, 50% cash)
        // Target: 40% NVDA, 60% USDC
        let pos = Position::new("NVDA", 50_000_000, 6, 100.0, 100.0).unwrap(); // $5,000 NVDA
        let target = AllocationTarget::new(vec![
            AssetWeight {
                symbol: "NVDA".to_string(),
                target_weight: BasisPoints(4_000), // 40%
            },
            AssetWeight {
                symbol: "USDC".to_string(),
                target_weight: BasisPoints(6_000), // 60%
            },
        ])
        .unwrap();

        let limits = PortfolioLimits {
            max_position_bps: BasisPoints(6_000),
            ..Default::default()
        };

        let portfolio = Portfolio::new(5_000.0, vec![pos], Some(target), limits).unwrap();

        let trades = portfolio
            .plan_rebalancing(BasisPoints(500)) // 5% drift threshold
            .expect("Rebalance planning succeeded");

        assert_eq!(trades.len(), 2);
        let nvda_trade = trades.iter().find(|t| t.symbol == "NVDA").unwrap();
        assert!(!nvda_trade.is_buy); // Sell NVDA
        assert_eq!(nvda_trade.current_value, 5_000_000_000);
        assert_eq!(nvda_trade.target_value, 4_000_000_000);
        assert_eq!(nvda_trade.trade_value, 1_000_000_000);

        let usdc_trade = trades.iter().find(|t| t.symbol == "USDC").unwrap();
        assert!(usdc_trade.is_buy); // Buy / add to USDC
        assert_eq!(usdc_trade.trade_value, 1_000_000_000);
    }
}
