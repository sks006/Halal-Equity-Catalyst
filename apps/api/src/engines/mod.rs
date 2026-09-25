//! Domain engines executing policy, risk, and trade decisions.

pub mod dbc_engine;
pub mod decision_engine;
pub mod execution_planner;
pub mod execution_recorder;
pub mod execution_signer;
pub mod pipeline_safety;
pub mod policy_engine;
pub mod risk_engine;

pub use dbc_engine::DbcEngine;
pub use decision_engine::{CanonicalExecutionPayload, DecisionEngine, ExecutionSigner};
pub use execution_planner::{
    ExecutionPlan, ExecutionPlanner, IdempotencyRecord, IdempotencyStatus, IdempotencyTracker,
    OracleReferenceInfo, PlannerError,
};
pub use execution_recorder::{
    ExecutionBalanceTracker, ExecutionRecord, ExecutionRecorderError,
};
pub use execution_signer::{
    DevTestSigner, ExternalSigner, KeypairSigner, RemoteHsmSigner, SignedTransaction, SignerError,
    TransactionBuilder, TransactionSignerService, TransactionSubmitter, UnavailableSigner,
    UnsignedTransaction,
};
pub use pipeline_safety::{
    PipelineExecutionAudit, PipelineExecutionContext, PipelineFailure, PipelineState,
    TradingPipelineSafetyGuard,
};
pub use policy_engine::{
    AllocationProposal, DeterministicPolicyAuthorizer, ExecutionAuthorization, PolicyEngine,
    PolicyRejectionReason, PolicyValidationOutcome, ValidationContext,
};
pub use risk_engine::RiskEngine;
