# Implementation Tasks & Phase Roadmap

This directory tracks the chronological implementation phases of Equity Catalyst. Each task follows the strict lifecycle:
`READ → PLAN → IMPLEMENT → TEST → REVIEW → UPDATE CURRENT STATE`.

---

## Task Ownership & Multi-Agent Concurrency

When running multiple AI coding agents concurrently, follow these strict coordination and ownership rules:

### 1. One Active Task Per Agent
Assign exactly **one** phase task per agent at any given time:
- **Agent 1** $\to$ `ai/tasks/phase-01-pyth.md`
- **Agent 2** $\to$ `ai/tasks/phase-02-asset-registry.md`
- **Agent 3** $\to$ `ai/tasks/phase-07-frontend.md`

### 2. Module Mutual Exclusion (No Simultaneous Edits)
- **Do not allow multiple agents to modify the same core module or directory simultaneously**.
- Respect the `Allowed Files` and `Forbidden Files` demarcated in each phase task file.

### 3. Canonical Sequential Implementation Order

```
PHASE 0 — Baseline & Safety
        ↓
PHASE 1 — Canonical Asset Registry
        ↓
PHASE 2 — Asset ↔ Pyth Feed Mapping
        ↓
PHASE 3 — Dynamic Subscription Model
        ↓
PHASE 4 — Pyth Real-Time SSE Client
        ↓
PHASE 5 — Dynamic Stream Manager
        ↓
PHASE 6 — Market Data Store
        ↓
PHASE 7 — Oracle Validation
        ↓
PHASE 8 — REST + WebSocket API
        ↓
PHASE 9 — DEX Quote Layer
        ↓
PHASE 10 — Deterministic Risk/Policy Engine
        ↓
PHASE 11 — Execution Planner
        ↓
PHASE 12 — Real Cryptographic Signer
        ↓
PHASE 13 — Anchor DEX CPI Execution
        ↓
PHASE 14 — Actual Execution Measurement
        ↓
PHASE 15 — Purification
        ↓
PHASE 16 — Failure/Recovery/Safety
        ↓
PHASE 17 — Integration Tests
        ↓
PHASE 18 — Production Hardening
```

### 4. Parallelization Topology

To maximize multi-agent throughput without module collision, the architecture supports parallel execution tracks:

```
                PHASE 00
                    │
          ┌─────────┼─────────┐
          ↓         ↓         ↓
       Pyth      Assets    Frontend
          │         │
          └────┬────┘
               ↓
           Market Engine
               │
         ┌─────┴─────┐
         ↓           ↓
      Meteora      Risk
         │           │
         └─────┬─────┘
               ↓
             Agent
               ↓
           Execution
               ↓
            Mainnet
```

### 5. AI Review Gate & Merge Protocol
- **Before moving from one phase to the next, check `ai/reviews/`**:
  1. The reviewer must be independent from the implementer where practical.
  2. The reviewer must verify the 8 gate questions:
     - Architecture correct?
     - Dependencies correct?
     - Tests adequate?
     - Security invariant preserved?
     - External API verified?
     - No secrets exposed?
     - No undocumented assumptions?
     - Mainnet impact understood?
  3. Merge and verify each phase (`cargo check`, `cargo test`) before the dependent agent starts.
  4. Update `ai/current-state.md`.

---

## Task Status System

Every phase specification file must declare its metadata and deliverable checklist at the top of the file:

```markdown
Status: NOT_STARTED
Owner: unassigned
Dependencies: None
Blocked by: None

## Checklist

- [ ] implementation
- [ ] unit tests
- [ ] integration tests
- [ ] verification
- [ ] documentation
- [ ] review
```

### Valid Statuses
- `NOT_STARTED`: Work has not begun.
- `IN_PROGRESS`: Actively owned and under development.
- `BLOCKED`: Blocked by an upstream phase dependency or external blocker.
- `READY_FOR_REVIEW`: Implementation and tests pass; pending formal review against `ai/invariants.md`.
- `DONE`: Fully merged, verified, and updated in `ai/current-state.md`.

---

## Phase Index

| Phase | Description | Status | Spec Document |
|---|---|---|---|
| **Phase 00** | Foundation: Shared crates, Anchor program, Docker, DB migrations | ✅ Completed | [phase-00-foundation.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-00-foundation.md) |
| **Phase 01** | Pyth Pro: Real-time equity oracle client and confidence scoring | ✅ Completed | [phase-01-pyth.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-01-pyth.md) |
| **Phase 02** | Asset Registry: Canonical asset verification (NVDAx, AAPLx, SPYx) | ✅ Completed | [phase-02-asset-registry.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-02-asset-registry.md) |
| **Phase 03** | Meteora DBC: Piecewise 3-regime discovery curve & SDK integration | ✅ Completed | [phase-03-meteora.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-03-meteora.md) |
| **Phase 04** | Equity Engine: Policy engine, 2-tier risk engine, decision engine | ✅ Completed | [phase-04-equity-engine.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-04-equity-engine.md) |
| **Phase 05** | Agent & Real Pool Launch: Real stock DBC pool launch & PostgreSQL persistence | ✅ Completed | [phase-05-agent.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-05-agent.md) |
| **Phase 06** | Jupiter Execution: Quote evaluation, slippage impact, and routing | ✅ Completed | [phase-06-execution.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-06-execution.md) |
| **Phase 07** | Frontend: Next.js App Router, Redux, Shadcn UI, light institutional aesthetic | ✅ Completed | [phase-07-frontend.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-07-frontend.md) |
| **Phase 08** | Mainnet & Devnet Validation: End-to-end transaction pipeline & on-chain verification | 🟡 Active | [phase-08-mainnet.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-08-mainnet.md) |
| **Phase 09** | Hardening: Circuit breakers, rate limiters, fuzzing, and stress testing | ⚪ Planned | [phase-09-hardening.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-09-hardening.md) |
| **Phase 10** | Hackathon Submission: Demo video, architectural walkthrough, and registry docs | ⚪ Planned | [phase-10-submission.md](file:///home/seam/Desktop/project/equity-catalyst/ai/tasks/phase-10-submission.md) |
