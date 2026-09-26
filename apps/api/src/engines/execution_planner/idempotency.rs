//! Idempotency tracker providing replay protection for execution plans.
//!
//! Prevents duplicate trade execution, multi-nonce replay of approved decisions,
//! and accidental double-dispatch of orders.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::planner::PlannerError;

/// Lifecycle status of an execution plan within the idempotency registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdempotencyStatus {
    /// Plan has been constructed and registered; awaiting execution dispatch
    Planned,
    /// Transaction payload created and dispatched to execution engine
    Executing,
    /// Transaction confirmed on-chain
    Completed,
    /// Transaction failed or was aborted
    Failed,
    /// Plan expired before execution
    Expired,
}

/// An audit entry in the idempotency registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    /// Primary idempotency key / nonce
    pub idempotency_key: String,
    /// Associated policy decision ID
    pub policy_decision_id: Uuid,
    /// Tamper-evident canonical plan digest
    pub plan_digest: String,
    /// Current lifecycle status
    pub status: IdempotencyStatus,
    /// Unix timestamp of creation
    pub created_at: i64,
    /// Unix timestamp of expiration
    pub expires_at: i64,
}

/// Thread-safe in-memory idempotency tracker.
#[derive(Debug, Default)]
pub struct IdempotencyTracker {
    records: RwLock<HashMap<String, IdempotencyRecord>>,
    decision_to_key: RwLock<HashMap<Uuid, String>>,
}

impl IdempotencyTracker {
    /// Creates a new, empty idempotency tracker.
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            decision_to_key: RwLock::new(HashMap::new()),
        }
    }

    /// Derives a standard idempotency key from a policy decision ID and client nonce.
    pub fn generate_key(policy_decision_id: &Uuid, nonce: &str) -> String {
        format!("idemp:{}:{}", policy_decision_id, nonce.trim())
    }

    /// Evaluates idempotency and registers a new execution plan record.
    ///
    /// Rules:
    /// 1. If the exact same `idempotency_key` is submitted with the exact same `plan_digest`,
    ///    this is treated as a valid idempotent replay and succeeds.
    /// 2. If the `idempotency_key` already exists with a DIFFERENT `plan_digest`, this is
    ///    a conflicting execution attempt and is rejected with `IdempotencyConflict`.
    /// 3. If the `policy_decision_id` has already been planned under a different key,
    ///    this prevents duplicate execution on the same decision and is rejected with
    ///    `DuplicateExecutionPlan`.
    pub async fn check_and_register(
        &self,
        record: IdempotencyRecord,
        current_time: i64,
    ) -> Result<(), PlannerError> {
        let mut records = self.records.write().await;
        let mut decisions = self.decision_to_key.write().await;

        // Check if key already exists
        if let Some(existing) = records.get(&record.idempotency_key) {
            if existing.plan_digest == record.plan_digest {
                // Exact replay
                if current_time >= existing.expires_at {
                    return Err(PlannerError::AuthorizationExpired {
                        decision_id: record.policy_decision_id,
                        valid_until: existing.expires_at,
                        current_time,
                    });
                }
                return Ok(());
            } else {
                // Conflicting plan payload under the same key
                return Err(PlannerError::IdempotencyConflict {
                    key: record.idempotency_key.clone(),
                    existing_digest: existing.plan_digest.clone(),
                    new_digest: record.plan_digest,
                });
            }
        }

        // Check if the policy decision ID was already used under another key
        if let Some(existing_key) = decisions.get(&record.policy_decision_id) {
            if let Some(existing_record) = records.get(existing_key) {
                // If the previous plan is still active (not Failed or Expired)
                if existing_record.status != IdempotencyStatus::Failed
                    && existing_record.status != IdempotencyStatus::Expired
                    && current_time < existing_record.expires_at
                {
                    return Err(PlannerError::DuplicateExecutionPlan {
                        decision_id: record.policy_decision_id,
                        existing_key: existing_key.clone(),
                    });
                }
            }
        }

        // Register new plan
        decisions.insert(record.policy_decision_id, record.idempotency_key.clone());
        records.insert(record.idempotency_key.clone(), record);
        Ok(())
    }

    /// Retrieves an existing idempotency record by key.
    pub async fn get_record(&self, key: &str) -> Option<IdempotencyRecord> {
        let records = self.records.read().await;
        records.get(key).cloned()
    }

    /// Updates the execution status of an idempotency record.
    pub async fn update_status(
        &self,
        key: &str,
        new_status: IdempotencyStatus,
    ) -> Result<(), PlannerError> {
        let mut records = self.records.write().await;
        if let Some(record) = records.get_mut(key) {
            record.status = new_status;
            Ok(())
        } else {
            Err(PlannerError::InvalidPlanParameter(format!(
                "Idempotency record not found for key: {}",
                key
            )))
        }
    }

    /// Number of active records in the tracker.
    pub async fn len(&self) -> usize {
        let records = self.records.read().await;
        records.len()
    }

    /// Returns whether the tracker is empty.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Prunes expired idempotency records from memory to prevent memory leaks in production.
    /// Returns the number of pruned records.
    pub async fn prune_expired(&self, current_time: i64) -> usize {
        let mut records = self.records.write().await;
        let mut decisions = self.decision_to_key.write().await;

        let before_count = records.len();
        let mut expired_keys = Vec::new();
        for (key, record) in records.iter() {
            if current_time >= record.expires_at {
                expired_keys.push((key.clone(), record.policy_decision_id));
            }
        }

        for (key, decision_id) in &expired_keys {
            records.remove(key);
            if decisions
                .get(decision_id)
                .map(|k| k == key)
                .unwrap_or(false)
            {
                decisions.remove(decision_id);
            }
        }

        before_count - records.len()
    }
}
