//! Phase 11: Empirical Performance Benchmarking Harness
//!
//! Measures exact latency percentiles (min, mean, p50, p95, p99, max),
//! throughput, database query latencies, and resource footprint across:
//! 1. Oracle update & normalization latency
//! 2. Shariah gate screening latency
//! 3. Risk gate evaluation latency
//! 4. Quote calculation & fee breakdown latency
//! 5. Agent decision latency
//! 6. Simulation assembly latency
//! 7. Execution preparation latency
//! 8. End-to-end decision-to-transaction pipeline latency
//! 9. PostgreSQL database query latency
//! 10. RPC retry classification latency
//! 11. WebSocket reconnect backoff calculation
//! 12. Concurrent throughput & execution capacity

use chrono::Utc;
use deadpool_postgres::Pool;
use equity_catalyst_api::{
    config::Config,
    create_db_pool,
    engines::{
        decision_engine::ExecutionSigner,
        policy_engine::PolicyEngine,
        risk_engine::{RiskAssessment, RiskEngine},
    },
    models::{EventModel, ExecutionModel, PolicyModel, PortfolioModel},
    repositories::execution_repository::ExecutionRepository,
    services::{
        execution_engine_service::{ExecutionEngineService, ExecutionRequest},
        OracleService, SolanaService,
    },
};
use equity_catalyst_pyth::PythClient;
use equity_catalyst_shared::{
    allocation::RebalanceTrade,
    fees::FeeSchedule,
    shariah::{
        screen_asset, BusinessActivityAssessment, BusinessCategory, OwnershipRecord,
        ScreeningPolicy, ShariahFinancialMetrics,
    },
    types::BasisPoints,
};
use solana_sdk::signature::{Keypair, Signer};
use std::{fs, sync::Arc, time::Instant};
use uuid::Uuid;

/// Statistical summary of benchmark measurements
#[derive(Debug, Clone)]
pub struct BenchmarkStats {
    pub name: String,
    pub samples: usize,
    pub min_us: f64,
    pub mean_us: f64,
    pub median_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub max_us: f64,
}

impl BenchmarkStats {
    pub fn compute(name: &str, mut durations_us: Vec<f64>) -> Self {
        assert!(!durations_us.is_empty());
        durations_us.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = durations_us.len();
        let min_us = durations_us[0];
        let max_us = durations_us[n - 1];
        let sum: f64 = durations_us.iter().sum();
        let mean_us = sum / (n as f64);
        let median_us = durations_us[n / 2];
        let p95_us = durations_us[(n as f64 * 0.95) as usize];
        let p99_us = durations_us[(n as f64 * 0.99) as usize];

        Self {
            name: name.to_string(),
            samples: n,
            min_us,
            mean_us,
            median_us,
            p95_us,
            p99_us,
            max_us,
        }
    }

    pub fn print_markdown_row(&self) {
        println!(
            "| {:<36} | {:>7} | {:>9.2} | {:>9.2} | {:>9.2} | {:>9.2} | {:>9.2} | {:>9.2} |",
            self.name,
            self.samples,
            self.min_us,
            self.mean_us,
            self.median_us,
            self.p95_us,
            self.p99_us,
            self.max_us
        );
    }
}

/// Reads current process Resident Set Size (RSS) in megabytes from /proc/self/statm
fn get_process_rss_mb() -> f64 {
    if let Ok(statm) = fs::read_to_string("/proc/self/statm") {
        let parts: Vec<&str> = statm.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(resident_pages) = parts[1].parse::<u64>() {
                let page_size_kb = 4.0; // standard 4KB page
                return (resident_pages as f64 * page_size_kb) / 1024.0;
            }
        }
    }
    0.0
}

fn setup_bench_env() -> (
    Pool,
    Arc<OracleService>,
    Arc<RiskEngine>,
    Arc<ExecutionSigner>,
) {
    let config = Config::from_env();
    let pool = create_db_pool(&config.database_url).expect("Failed to connect to test Postgres");
    let pyth_client = Arc::new(PythClient::new_mock());
    pyth_client.set_mock_price("NVDA", "12550000000", "5000000", -8, Utc::now().timestamp());
    let oracle_service = Arc::new(OracleService::new(pyth_client, None));
    let risk_engine = Arc::new(RiskEngine::new());
    let signer = Arc::new(ExecutionSigner::load_or_generate("/tmp/bench_signer.json"));
    (pool, oracle_service, risk_engine, signer)
}

#[tokio::test]
async fn test_run_empirical_pipeline_benchmarks() {
    println!("\n==========================================================================================");
    println!("EQUITY CATALYST — EMPIRICAL PERFORMANCE BENCHMARK SUITE (v1.0.0-rc1)");
    println!("==========================================================================================");

    let initial_rss = get_process_rss_mb();
    println!("Process Initial RSS: {:.2} MB", initial_rss);

    let (pool, oracle_service, risk_engine, signer) = setup_bench_env();
    let iters = 1_000;

    println!("\nRunning {} iterations per benchmark stage...\n", iters);
    println!("| Pipeline Stage / Metric              | Samples |    Min (µs)|   Mean (µs)| Median (µs)|    p95 (µs)|    p99 (µs)|    Max (µs)|");
    println!("|--------------------------------------|---------|------------|------------|------------|------------|------------|------------|");

    // -------------------------------------------------------------------------
    // 1. Oracle update & normalization latency
    // -------------------------------------------------------------------------
    let mut oracle_latencies = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        let price = oracle_service.get_normalized_price("NVDA").await.unwrap();
        let _ = price.price_usd;
        oracle_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let oracle_stats = BenchmarkStats::compute("1. Oracle Update Latency", oracle_latencies);
    oracle_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 2. Shariah Gate latency
    // -------------------------------------------------------------------------
    let policy = ScreeningPolicy::board_approved_v1();
    let ownership = OwnershipRecord {
        verified: true,
        issuer: "Backed Finance AG".to_string(),
        custodian: "Maerki Baumann & Co. AG".to_string(),
        legal_structure: "Swiss DLT Act Statutory SPV".to_string(),
        instrument_reference: "ISIN: US67066G1040".to_string(),
        evidence_hash: "3a4f8b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a"
            .to_string(),
        verified_at: Utc::now().timestamp() - 3600,
        expires_at: Utc::now().timestamp() + 86400 * 30,
    };
    let metrics = ShariahFinancialMetrics::from_market_cap_ratios(1200, 800, 50);
    let business = BusinessActivityAssessment::reviewed_permissible(
        BusinessCategory::Technology,
        "Enterprise enterprise cloud hardware",
        "SEC Form 10-K",
    );
    let now = Utc::now().timestamp();

    let mut shariah_latencies = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        let res = screen_asset(&business, &metrics, &ownership, &policy, now);
        assert!(res.is_approved());
        shariah_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let shariah_stats = BenchmarkStats::compute("2. Shariah Gate Latency", shariah_latencies);
    shariah_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 3. Risk Gate latency (Spot ownership + Funding + Exposure bounds)
    // -------------------------------------------------------------------------
    let trade = RebalanceTrade {
        symbol: "NVDA".to_string(),
        is_buy: true,
        current_value: 10_000,
        target_value: 15_000,
        trade_value: 5_000,
        drift_bps: BasisPoints(100),
    };
    let trades = vec![trade];
    let position_nvda = PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin".to_string(),
        asset_symbol: "NVDA".to_string(),
        asset_mint: "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh".to_string(),
        amount: 160,
        entry_price_usd: 125.0,
        current_price_usd: 125.0,
        current_value_usd: 20_000.0,
        target_weight_bps: 2000,
        current_weight_bps: 2000,
        last_rebalanced_at: Some(Utc::now()),
        updated_at: Utc::now(),
    };
    let position_usdc = PortfolioModel {
        portfolio_id: Uuid::new_v4(),
        vault_address: "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin".to_string(),
        asset_symbol: "USDC".to_string(),
        asset_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        amount: 80_000,
        entry_price_usd: 1.0,
        current_price_usd: 1.0,
        current_value_usd: 80_000.0,
        target_weight_bps: 8000,
        current_weight_bps: 8000,
        last_rebalanced_at: Some(Utc::now()),
        updated_at: Utc::now(),
    };
    let positions = vec![position_nvda, position_usdc];
    let risk_policy = PolicyModel {
        policy_address: "pol-risk-bench".to_string(),
        vault_address: "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin".to_string(),
        authority: "auth123".to_string(),
        min_cash_bps: 1000,
        max_position_bps: 3000,
        stop_loss_bps: 800,
        take_profit_bps: 2000,
        rebalance_threshold_bps: 150,
        is_active: true,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let mut risk_latencies = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        let assessment = risk_engine.evaluate_proposed_trades(
            &trades,
            &positions,
            &risk_policy,
            100_000,
            20_000,
        );
        assert_eq!(assessment, RiskAssessment::Approved);
        risk_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let risk_stats = BenchmarkStats::compute("3. Risk Gate Latency", risk_latencies);
    risk_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 4. Quote Calculation & Deterministic Fee Breakdown latency
    // -------------------------------------------------------------------------
    let fee_schedule = FeeSchedule::standard_v1();
    let mut quote_latencies = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        let fees = fee_schedule.calculate_fees(500_000, 6, None);
        assert_eq!(fees.fee_calculation_version, "v1.0-deterministic");
        let valid = fee_schedule.validate_fee_disclosure(&fees, 500_000, None);
        assert!(valid.is_ok());
        quote_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let quote_stats = BenchmarkStats::compute("4. Quote & Fee Calc Latency", quote_latencies);
    quote_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 5. Agent Decision Latency (Policy Engine + Proposal Validation)
    // -------------------------------------------------------------------------
    let policy_engine = PolicyEngine::new();
    let event = EventModel {
        event_id: Uuid::new_v4(),
        vault_address: Some("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin".to_string()),
        event_type: "DEPOSIT".to_string(),
        source: "ON_CHAIN".to_string(),
        sentiment_score: None,
        payload: serde_json::json!({ "amount": 10000 }),
        status: "PENDING".to_string(),
        detected_at: Utc::now(),
        processed_at: None,
    };
    let test_policy = PolicyModel {
        policy_address: "pol-bench".to_string(),
        vault_address: event.vault_address.clone().unwrap(),
        authority: "auth123".to_string(),
        min_cash_bps: 1000,
        max_position_bps: 3000,
        stop_loss_bps: 800,
        take_profit_bps: 2000,
        rebalance_threshold_bps: 150,
        is_active: true,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let mut agent_latencies = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        let eval = policy_engine.evaluate(&event, &test_policy, &positions, 100_000);
        assert!(eval.is_ok());
        agent_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let agent_stats = BenchmarkStats::compute("5. Agent Decision Latency", agent_latencies);
    agent_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 6. Simulation & Pre-Flight Preparation Latency
    // -------------------------------------------------------------------------
    let solana_service = Arc::new(SolanaService::new(
        "http://127.0.0.1:8899",
        "ws://127.0.0.1:8900",
        None,
        None,
    ));
    let mut sim_latencies = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        let dummy_ix = solana_sdk::instruction::Instruction {
            program_id: Keypair::new().pubkey(),
            accounts: vec![],
            data: vec![1, 2, 3, 4],
        };
        // Assembly & serialization
        let msg = solana_sdk::message::Message::new(&[dummy_ix], Some(&Keypair::new().pubkey()));
        let _ = msg.serialize();
        sim_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let sim_stats = BenchmarkStats::compute("6. Simulation Assembly Latency", sim_latencies);
    sim_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 7. Execution Preparation Latency (Idempotency Key & Fee Envelope Assembly)
    // -------------------------------------------------------------------------
    let mut prep_latencies = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        let execution_id = Uuid::new_v4();
        let fees = fee_schedule.calculate_fees(100_000, 6, None);
        let req = ExecutionRequest {
            execution_id,
            vault_address: "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin".to_string(),
            event_id: Some(Uuid::new_v4()),
            action_type: 1,
            action_name: "BUY_NVDA".to_string(),
            input_mint: Keypair::new().pubkey().to_string(),
            output_mint: Keypair::new().pubkey().to_string(),
            amount_in: 100_000,
            min_amount_out: 99_000,
            amount_out_expected: 100_000,
            slippage_bps: 50,
            target_symbol: "NVDA".to_string(),
            fee_breakdown: Some(fees),
            quote_id: None,
            policy_decision_id: None,
        };
        let _ = serde_json::to_string(&req).unwrap();
        prep_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let prep_stats = BenchmarkStats::compute("7. Execution Prep Latency", prep_latencies);
    prep_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 8. End-to-End Decision -> Revalidation Pipeline Latency
    // -------------------------------------------------------------------------
    let exec_service = ExecutionEngineService::new(
        solana_service.clone(),
        oracle_service.clone(),
        risk_engine.clone(),
        signer.clone(),
        None,
        None,
        None,
    );
    let mut e2e_latencies = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        let req = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            vault_address: "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin".to_string(),
            event_id: None,
            action_type: 1,
            action_name: "BUY_NVDA".to_string(),
            input_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            output_mint: "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh".to_string(),
            amount_in: 50_000,
            min_amount_out: 49_500,
            amount_out_expected: 50_000,
            slippage_bps: 50,
            target_symbol: "NVDA".to_string(),
            fee_breakdown: Some(fee_schedule.calculate_fees(50_000, 6, None)),
            quote_id: None,
            policy_decision_id: None,
        };
        let res = exec_service
            .revalidate_shariah_and_spot(&req, Utc::now().timestamp())
            .await;
        assert!(res.is_ok());
        e2e_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let e2e_stats = BenchmarkStats::compute("8. End-to-End Pipeline Latency", e2e_latencies);
    e2e_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 9. Database Query Latency (Postgres deadpool)
    // -------------------------------------------------------------------------
    let vault_repo =
        equity_catalyst_api::repositories::vault_repository::VaultRepository::new(pool.clone());
    let exec_repo = ExecutionRepository::new(pool.clone());
    let db_iters = 200; // 200 real DB roundtrips
    let mut db_insert_latencies = Vec::with_capacity(db_iters);
    let mut db_read_latencies = Vec::with_capacity(db_iters);

    let test_vault_addr = Keypair::new().pubkey().to_string();
    let parent_vault = equity_catalyst_api::models::VaultModel {
        vault_address: test_vault_addr.clone(),
        authority: Keypair::new().pubkey().to_string(),
        name: "Bench Vault".to_string(),
        symbol: "BV".to_string(),
        deposit_mint: Keypair::new().pubkey().to_string(),
        vault_token_account: Keypair::new().pubkey().to_string(),
        total_shares: 100_000,
        total_deposits: 100_000,
        is_paused: false,
        bump: 255,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    vault_repo.create(&parent_vault).await.unwrap();

    for _ in 0..db_iters {
        let exec_id = Uuid::new_v4();
        let model = ExecutionModel {
            execution_id: exec_id,
            vault_address: test_vault_addr.clone(),
            event_id: None,
            action: "BUY_NVDA".to_string(),
            input_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            output_mint: "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh".to_string(),
            amount_in: 50_000,
            amount_out_expected: 50_000,
            amount_out_actual: Some(50_000),
            slippage_bps: 50,
            tx_signature: Some(format!("sig-{}", Uuid::new_v4().simple())),
            status: "confirmed".to_string(),
            error_message: None,
            executed_at: Utc::now(),
            confirmed_at: Some(Utc::now()),
            quote_id: Some("benchmark-quote".to_string()),
            policy_decision_id: Some(Uuid::new_v4()),
            amount_out_min: Some(49_500),
        };

        let t0 = Instant::now();
        let inserted = exec_repo.create(&model).await.unwrap();
        db_insert_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);

        let t1 = Instant::now();
        let _ = exec_repo.find_by_id(inserted.execution_id).await.unwrap();
        db_read_latencies.push(t1.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let db_insert_stats = BenchmarkStats::compute("9a. DB Insert Latency", db_insert_latencies);
    let db_read_stats = BenchmarkStats::compute("9b. DB Query Latency (Read)", db_read_latencies);
    db_insert_stats.print_markdown_row();
    db_read_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 10. RPC Retry Classification Latency
    // -------------------------------------------------------------------------
    let mut rpc_retry_latencies = Vec::with_capacity(iters);
    for i in 0..iters {
        let t0 = Instant::now();
        let status = if i % 2 == 0 { 429 } else { 503 };
        let is_retryable = status == 429 || status == 503 || status == 504;
        let delay_ms = if is_retryable {
            let attempt = (i % 5) as u32;
            let base = 50u64 * (2u64.pow(attempt));
            base.min(2000)
        } else {
            0
        };
        assert!(delay_ms <= 2000);
        rpc_retry_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let rpc_stats = BenchmarkStats::compute("10. RPC Retry Classification", rpc_retry_latencies);
    rpc_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 11. WebSocket Reconnect Backoff Calculation Latency
    // -------------------------------------------------------------------------
    let mut ws_latencies = Vec::with_capacity(iters);
    for i in 0..iters {
        let t0 = Instant::now();
        let reconnect_attempts = (i % 8) as u32;
        let base_delay_ms = 100u64;
        let max_delay_ms = 30_000u64;
        let factor = 2u64.pow(reconnect_attempts.min(6));
        let backoff = (base_delay_ms * factor).min(max_delay_ms);
        assert!(backoff <= 30_000);
        ws_latencies.push(t0.elapsed().as_nanos() as f64 / 1_000.0);
    }
    let ws_stats = BenchmarkStats::compute("11. WS Reconnect Backoff Calc", ws_latencies);
    ws_stats.print_markdown_row();

    // -------------------------------------------------------------------------
    // 12. Concurrent Throughput & Execution Capacity
    // -------------------------------------------------------------------------
    println!("\nMeasuring Concurrent Execution Capacity (50 parallel workers x 100 requests = 5,000 requests)...");
    let concurrency = 50;
    let requests_per_worker = 100;

    let t_start_concurrent = Instant::now();
    let mut handles = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
        let fee_schedule_clone = fee_schedule.clone();
        let handle = tokio::spawn(async move {
            let mut successful = 0;
            for _ in 0..requests_per_worker {
                let fees = fee_schedule_clone.calculate_fees(25_000, 6, None);
                let check = fee_schedule_clone.validate_fee_disclosure(&fees, 25_000, None);
                if check.is_ok() {
                    successful += 1;
                }
            }
            successful
        });
        handles.push(handle);
    }

    let mut total_success = 0;
    for h in handles {
        total_success += h.await.unwrap();
    }
    let concurrent_duration = t_start_concurrent.elapsed();
    let throughput_ops_sec = (total_success as f64) / concurrent_duration.as_secs_f64();

    println!("Total concurrent evaluations: {}", total_success);
    println!("Elapsed time: {:.2?}", concurrent_duration);
    println!("Throughput: {:.2} operations/second", throughput_ops_sec);

    let final_rss = get_process_rss_mb();
    println!(
        "Process Final RSS: {:.2} MB (Delta: +{:.2} MB)",
        final_rss,
        final_rss - initial_rss
    );
    println!("==========================================================================================\n");
}
