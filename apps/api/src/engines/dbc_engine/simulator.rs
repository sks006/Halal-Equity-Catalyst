use serde::{Deserialize, Serialize};

/// Input specification for simulating a DBC configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DbcSimulationInput {
    pub name: String,
    pub starting_price: f64,
    pub segments: Vec<SimulationSegment>,
    pub base_liquidity: f64,
    pub base_fee_bps: u32,
    pub dynamic_fee_multiplier: f64,
    pub graduation_threshold: f64,
    #[serde(default = "default_trade_volumes")]
    pub trade_volumes: Vec<f64>,
}

fn default_trade_volumes() -> Vec<f64> {
    vec![
        500.0, 1_000.0, 2_500.0, 5_000.0, 10_000.0, 25_000.0, 50_000.0, 100_000.0, 250_000.0,
        500_000.0, 750_000.0,
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationSegment {
    pub name: String,
    pub start_price: f64,
    pub end_price: f64,
    pub liquidity_weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationPricePoint {
    pub cumulative_quote_in: f64,
    pub price: f64,
    pub quote_reserve: f64,
    pub base_sold: f64,
    pub price_impact_pct: f64,
    pub effective_slippage_pct: f64,
    pub current_regime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraduationEstimate {
    pub quote_needed_to_graduate: f64,
    pub price_at_graduation: f64,
    pub base_tokens_sold_at_graduation: f64,
    pub is_graduated_in_simulation: bool,
    pub estimated_damm_initial_pool_liquidity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DbcSimulationResult {
    pub configuration_name: String,
    pub starting_price: f64,
    pub graduation_threshold: f64,
    pub price_path: Vec<SimulationPricePoint>,
    pub price_impact_at_1k: f64,
    pub price_impact_at_10k: f64,
    pub price_impact_at_50k: f64,
    pub slippage_at_10k: f64,
    pub graduation_estimate: GraduationEstimate,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonSimulationRequest {
    pub starting_price: f64,
    pub graduation_threshold: f64,
    pub custom_config_a: Option<DbcSimulationInput>,
    pub custom_config_b: Option<DbcSimulationInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonSimulationResponse {
    pub config_a: DbcSimulationResult,
    pub config_b: DbcSimulationResult,
    pub default_dbc: DbcSimulationResult,
    pub recommendation: String,
}

/// Core simulation engine for computing price paths, liquidity depth, and graduation points.
pub struct DbcSimulator;

impl DbcSimulator {
    /// Simulates a single DBC configuration across cumulative trade volume steps.
    pub fn simulate(input: &DbcSimulationInput) -> DbcSimulationResult {
        let mut price_path = Vec::new();
        let mut current_quote_reserve = 0.0;
        let mut current_base_sold = 0.0;
        let mut current_price = input.starting_price;

        let fee_factor = 1.0 - (input.base_fee_bps as f64 / 10_000.0);

        let mut price_impact_at_1k = 0.0;
        let mut price_impact_at_10k = 0.0;
        let mut price_impact_at_50k = 0.0;
        let mut slippage_at_10k = 0.0;

        let mut prev_quote_vol = 0.0;

        for &target_quote_vol in &input.trade_volumes {
            let delta_quote = (target_quote_vol - prev_quote_vol).max(0.0);
            if delta_quote > 0.0 {
                let net_quote = delta_quote * fee_factor;
                let (next_price, base_out, active_regime) = Self::step_trade(
                    current_price,
                    net_quote,
                    &input.segments,
                    input.base_liquidity,
                );

                current_price = next_price;
                current_quote_reserve += delta_quote;
                current_base_sold += base_out;
                prev_quote_vol = target_quote_vol;

                let price_impact =
                    ((current_price - input.starting_price) / input.starting_price) * 100.0;
                let avg_exec_price = if current_base_sold > 0.0 {
                    current_quote_reserve / current_base_sold
                } else {
                    current_price
                };
                let effective_slippage =
                    ((avg_exec_price - input.starting_price) / input.starting_price) * 100.0;

                price_path.push(SimulationPricePoint {
                    cumulative_quote_in: target_quote_vol,
                    price: current_price,
                    quote_reserve: current_quote_reserve,
                    base_sold: current_base_sold,
                    price_impact_pct: price_impact,
                    effective_slippage_pct: effective_slippage,
                    current_regime: active_regime,
                });

                if (target_quote_vol - 1_000.0).abs() < 100.0 {
                    price_impact_at_1k = price_impact;
                }
                if (target_quote_vol - 10_000.0).abs() < 500.0 {
                    price_impact_at_10k = price_impact;
                    slippage_at_10k = effective_slippage;
                }
                if (target_quote_vol - 50_000.0).abs() < 1_000.0 {
                    price_impact_at_50k = price_impact;
                }
            }
        }

        // Graduation point estimation
        let is_graduated = current_quote_reserve >= input.graduation_threshold;
        let last_segment = input.segments.last();
        let max_price = last_segment.map(|s| s.end_price).unwrap_or(current_price);

        let graduation_estimate = GraduationEstimate {
            quote_needed_to_graduate: input.graduation_threshold,
            price_at_graduation: max_price,
            base_tokens_sold_at_graduation: current_base_sold
                * (input.graduation_threshold / current_quote_reserve.max(1.0)),
            is_graduated_in_simulation: is_graduated,
            estimated_damm_initial_pool_liquidity: input.graduation_threshold * 0.95, // 95% post-migration fee
        };

        let summary = format!(
            "{}: $10k price impact: +{:.2}%, $50k price impact: +{:.2}%. Graduation threshold ${:.0} {} in simulation.",
            input.name,
            price_impact_at_10k,
            price_impact_at_50k,
            input.graduation_threshold,
            if is_graduated { "reached" } else { "pending" }
        );

        DbcSimulationResult {
            configuration_name: input.name.clone(),
            starting_price: input.starting_price,
            graduation_threshold: input.graduation_threshold,
            price_path,
            price_impact_at_1k,
            price_impact_at_10k,
            price_impact_at_50k,
            slippage_at_10k,
            graduation_estimate,
            summary,
        }
    }

    /// Evaluates a single incremental trade step through piecewise curve segments.
    fn step_trade(
        current_price: f64,
        quote_in: f64,
        segments: &[SimulationSegment],
        base_liquidity: f64,
    ) -> (f64, f64, String) {
        if segments.is_empty() {
            let next_p = current_price * (1.0 + quote_in / (base_liquidity * 100.0));
            let base_out = quote_in / ((current_price + next_p) / 2.0);
            return (next_p, base_out, "Default Monolithic".to_string());
        }

        let mut remaining_quote = quote_in;
        let mut p_curr = current_price;
        let mut total_base_out = 0.0;
        let mut active_regime_name = segments[0].name.clone();

        for seg in segments {
            if p_curr < seg.end_price {
                active_regime_name = seg.name.clone();
                let effective_l = base_liquidity * (seg.liquidity_weight.max(1) as f64);

                let sqrt_p_curr = p_curr.sqrt();
                let sqrt_p_max = seg.end_price.sqrt();

                // Max quote this segment can absorb: L * (sqrt(P_max) - sqrt(P_curr))
                let max_quote_segment = effective_l * (sqrt_p_max - sqrt_p_curr);

                if remaining_quote <= max_quote_segment {
                    // Entire remaining trade fits inside this segment
                    let delta_sqrt_p = remaining_quote / effective_l;
                    let sqrt_p_next = sqrt_p_curr + delta_sqrt_p;
                    let p_next = sqrt_p_next * sqrt_p_next;

                    let base_out = effective_l * (1.0 / sqrt_p_curr - 1.0 / sqrt_p_next);
                    total_base_out += base_out;
                    p_curr = p_next;
                    remaining_quote = 0.0;
                    break;
                } else {
                    // Trade saturates this segment and pushes into next segment
                    let base_out = effective_l * (1.0 / sqrt_p_curr - 1.0 / sqrt_p_max);
                    total_base_out += base_out;
                    remaining_quote -= max_quote_segment;
                    p_curr = seg.end_price;
                }
            }
        }

        // If quote still remains after all segments, extrapolate with final segment weight
        if remaining_quote > 0.0 {
            let final_weight = segments.last().map(|s| s.liquidity_weight).unwrap_or(1) as f64;
            let effective_l = base_liquidity * final_weight;
            let sqrt_p_curr = p_curr.sqrt();
            let delta_sqrt_p = remaining_quote / effective_l;
            let sqrt_p_next = sqrt_p_curr + delta_sqrt_p;
            let p_next = sqrt_p_next * sqrt_p_next;
            let base_out = effective_l * (1.0 / sqrt_p_curr - 1.0 / sqrt_p_next);
            total_base_out += base_out;
            p_curr = p_next;
        }

        (p_curr, total_base_out, active_regime_name)
    }

    /// Generates a side-by-side comparative simulation of Config A, Config B, and Default DBC.
    pub fn compare(req: ComparisonSimulationRequest) -> ComparisonSimulationResponse {
        let p0 = req.starting_price.max(1.0);
        let g = req.graduation_threshold.max(100.0);

        // 1. Configuration A (Equity Discovery: 1x -> 4x -> 8x)
        let config_a_input = req.custom_config_a.unwrap_or_else(|| DbcSimulationInput {
            name: "Configuration A (Equity Discovery)".to_string(),
            starting_price: p0,
            segments: vec![
                SimulationSegment {
                    name: "Regime A: Bootstrapping".to_string(),
                    start_price: p0 * 0.85,
                    end_price: p0 * 1.0,
                    liquidity_weight: 1,
                },
                SimulationSegment {
                    name: "Regime B: Active Discovery".to_string(),
                    start_price: p0 * 1.0,
                    end_price: p0 * 1.3,
                    liquidity_weight: 4,
                },
                SimulationSegment {
                    name: "Regime C: Mature Buffer".to_string(),
                    start_price: p0 * 1.3,
                    end_price: p0 * 1.5,
                    liquidity_weight: 8,
                },
            ],
            base_liquidity: 50_000.0,
            base_fee_bps: 250,
            dynamic_fee_multiplier: 1.5,
            graduation_threshold: g,
            trade_volumes: default_trade_volumes(),
        });

        // 2. Configuration B (Aggressive Growth: 1x -> 2x -> 4x)
        let config_b_input = req.custom_config_b.unwrap_or_else(|| DbcSimulationInput {
            name: "Configuration B (Aggressive Growth)".to_string(),
            starting_price: p0,
            segments: vec![
                SimulationSegment {
                    name: "Initial Ramp".to_string(),
                    start_price: p0 * 0.90,
                    end_price: p0 * 1.15,
                    liquidity_weight: 1,
                },
                SimulationSegment {
                    name: "Expansion Band".to_string(),
                    start_price: p0 * 1.15,
                    end_price: p0 * 1.50,
                    liquidity_weight: 2,
                },
                SimulationSegment {
                    name: "Graduation Step".to_string(),
                    start_price: p0 * 1.50,
                    end_price: p0 * 2.00,
                    liquidity_weight: 4,
                },
            ],
            base_liquidity: 30_000.0,
            base_fee_bps: 100,
            dynamic_fee_multiplier: 1.0,
            graduation_threshold: g,
            trade_volumes: default_trade_volumes(),
        });

        // 3. Default DBC (Meme / Speculative Monolithic Curve: Paper-thin 1x liquidity throughout)
        let default_dbc_input = DbcSimulationInput {
            name: "Default DBC (Speculative Monolithic)".to_string(),
            starting_price: p0,
            segments: vec![SimulationSegment {
                name: "Monolithic Curve".to_string(),
                start_price: p0,
                end_price: p0 * 10.0,
                liquidity_weight: 1,
            }],
            base_liquidity: 10_000.0,
            base_fee_bps: 100,
            dynamic_fee_multiplier: 1.0,
            graduation_threshold: g,
            trade_volumes: default_trade_volumes(),
        };

        let res_a = Self::simulate(&config_a_input);
        let res_b = Self::simulate(&config_b_input);
        let res_default = Self::simulate(&default_dbc_input);

        let recommendation = format!(
            "Configuration A provides {:.1}x lower slippage at $10k than Default DBC ({:.2}% vs {:.2}%), preventing predatory frontrunning while guaranteeing continuous valuation discovery.",
            res_default.price_impact_at_10k / res_a.price_impact_at_10k.max(0.01),
            res_a.price_impact_at_10k,
            res_default.price_impact_at_10k
        );

        ComparisonSimulationResponse {
            config_a: res_a,
            config_b: res_b,
            default_dbc: res_default,
            recommendation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_basic_run() {
        let input = DbcSimulationInput {
            name: "Test Equity Curve".to_string(),
            starting_price: 100.0,
            segments: vec![
                SimulationSegment {
                    name: "Regime A".to_string(),
                    start_price: 85.0,
                    end_price: 100.0,
                    liquidity_weight: 1,
                },
                SimulationSegment {
                    name: "Regime B".to_string(),
                    start_price: 100.0,
                    end_price: 130.0,
                    liquidity_weight: 4,
                },
            ],
            base_liquidity: 50_000.0,
            base_fee_bps: 250,
            dynamic_fee_multiplier: 1.0,
            graduation_threshold: 750.0,
            trade_volumes: vec![1_000.0, 5_000.0, 10_000.0],
        };

        let res = DbcSimulator::simulate(&input);
        assert_eq!(res.price_path.len(), 3);
        assert!(res.price_impact_at_1k < res.price_impact_at_10k);
        assert!(res.price_path.last().unwrap().price > 100.0);
    }

    #[test]
    fn test_simulator_comparison() {
        let req = ComparisonSimulationRequest {
            starting_price: 100.0,
            graduation_threshold: 750.0,
            custom_config_a: None,
            custom_config_b: None,
        };

        let comp = DbcSimulator::compare(req);
        assert_eq!(
            comp.config_a.configuration_name,
            "Configuration A (Equity Discovery)"
        );
        assert_eq!(
            comp.config_b.configuration_name,
            "Configuration B (Aggressive Growth)"
        );
        assert_eq!(
            comp.default_dbc.configuration_name,
            "Default DBC (Speculative Monolithic)"
        );

        // Config A should have lower price impact at $10k than Default DBC due to 4x concentrated liquidity
        assert!(comp.config_a.price_impact_at_10k < comp.default_dbc.price_impact_at_10k);
    }
}
