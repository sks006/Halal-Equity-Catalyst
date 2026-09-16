//! Allocation calculation translating policy signals into proposed rebalance trade orders.

use equity_catalyst_shared::{
    allocation::{calculate_rebalance_plan, RebalanceTrade},
    constants::MAX_BPS,
    types::{AllocationTarget, AssetWeight, BasisPoints, PositionSnapshot, SignalType},
};

use super::signals::PolicySignal;
use crate::{error::ApiError, models::PortfolioModel};

/// Adjusts target portfolio weights according to the policy signal and generates proposed trades.
pub fn calculate_target_allocation(
    signal: &PolicySignal,
    positions: &[PortfolioModel],
    total_portfolio_usd: u64,
    rebalance_threshold_bps: u16,
) -> Result<(AllocationTarget, Vec<RebalanceTrade>), ApiError> {
    if positions.is_empty() {
        return Err(ApiError::BadRequest(
            "Portfolio has no active positions".to_string(),
        ));
    }

    if signal.signal_type == SignalType::EmergencyExit {
        // In emergency halt, propose selling all non-USDC assets into cash
        let mut trades = Vec::new();
        for pos in positions {
            if pos.asset_symbol != "USDC" && pos.current_value_usd > 0.0 {
                trades.push(RebalanceTrade {
                    symbol: pos.asset_symbol.clone(),
                    is_buy: false,
                    current_value: pos.current_value_usd as u64,
                    target_value: 0,
                    trade_value: pos.current_value_usd as u64,
                    drift_bps: BasisPoints(pos.current_weight_bps as u16),
                });
            }
        }
        let cash_target = AllocationTarget::new(vec![AssetWeight {
            symbol: "USDC".to_string(),
            target_weight: BasisPoints(MAX_BPS),
        }])?;
        return Ok((cash_target, trades));
    }

    // Build base target weights from current portfolio
    let mut weights: Vec<AssetWeight> = positions
        .iter()
        .map(|p| AssetWeight {
            symbol: p.asset_symbol.clone(),
            target_weight: BasisPoints(p.target_weight_bps as u16),
        })
        .collect();

    // If signal specifies a weight delta for an asset, apply it and balance with USDC
    if let Some(ref target_sym) = signal.symbol {
        let delta = signal.weight_delta_bps;
        if delta != 0 {
            let mut target_idx = None;
            let mut usdc_idx = None;

            for (i, w) in weights.iter().enumerate() {
                if w.symbol == *target_sym {
                    target_idx = Some(i);
                }
                if w.symbol == "USDC" {
                    usdc_idx = Some(i);
                }
            }

            if let (Some(t_idx), Some(u_idx)) = (target_idx, usdc_idx) {
                let current_target = weights[t_idx].target_weight.0 as i32;
                let current_usdc = weights[u_idx].target_weight.0 as i32;

                let new_target = (current_target + delta as i32).clamp(0, MAX_BPS as i32) as u16;
                let new_usdc = (current_usdc - delta as i32).clamp(0, MAX_BPS as i32) as u16;

                weights[t_idx].target_weight = BasisPoints(new_target);
                weights[u_idx].target_weight = BasisPoints(new_usdc);
            }
        }
    }

    // Ensure total sum equals exactly 10,000 bps
    let total_sum: u32 = weights.iter().map(|w| w.target_weight.0 as u32).sum();
    if total_sum != MAX_BPS as u32 {
        // Normalize any rounding discrepancy to USDC
        if let Some(usdc) = weights.iter_mut().find(|w| w.symbol == "USDC") {
            let diff = MAX_BPS as i32 - total_sum as i32;
            let adjusted = (usdc.target_weight.0 as i32 + diff).max(0) as u16;
            usdc.target_weight = BasisPoints(adjusted);
        }
    }

    let allocation_target = AllocationTarget::new(weights)?;

    // Map positions to shared PositionSnapshot
    let snapshots: Vec<PositionSnapshot> = positions
        .iter()
        .map(|p| PositionSnapshot {
            symbol: p.asset_symbol.clone(),
            current_amount: p.amount,
            entry_price_usd: (p.entry_price_usd * 100.0) as u64,
            current_price_usd: (p.current_price_usd * 100.0) as u64,
            current_value_usd: p.current_value_usd as u64,
        })
        .collect();

    // Compute concrete rebalance trades using pure shared engine
    let trades = calculate_rebalance_plan(
        &snapshots,
        &allocation_target,
        total_portfolio_usd,
        BasisPoints(rebalance_threshold_bps),
    )?;

    Ok((allocation_target, trades))
}
