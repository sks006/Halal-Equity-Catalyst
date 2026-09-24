//! Execution Planner module converting approved allocation decisions into deterministic execution plans.
//!
//! # Security Constraints
//! The planner is strictly non-executing and non-signing:
//! - NO transaction signing
//! - NO private key access
//! - NO transaction broadcasting or RPC submission
//! - NO risk or Shariah bypass
//! - NO modification of approved quantities

pub mod idempotency;
pub mod plan;
pub mod planner;

pub use idempotency::{IdempotencyRecord, IdempotencyStatus, IdempotencyTracker};
pub use plan::{ExecutionPlan, OracleReferenceInfo};
pub use planner::{ExecutionPlanner, PlannerError};
