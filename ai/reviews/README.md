# System Reviews & Audits

This directory contains formal engineering reviews across architecture, security, integrations, and submission readiness.

---

## AI Review Gate

Before any phase can transition from `READY_FOR_REVIEW` to `DONE`, an AI reviewer must evaluate the changes and record a review gate assessment.

### Reviewer Independence
- **The reviewer must NOT be the same agent that implemented the task where practical**.
- The reviewer acts as an adversarial auditor verifying correctness, security, and integration stability.

### The 8 Reviewer Questions
Every review gate evaluation must explicitly answer:

1. **Architecture correct?** — Does the implementation respect module boundaries in `ai/module-map.md` and dependencies in `ai/dependency-map.md`?
2. **Dependencies correct?** — Are there any forbidden imports, circular dependencies, or domain-leaking wire types?
3. **Tests adequate?** — Are normal flows, failure paths, and edge cases (e.g. division by zero, stale timestamps) thoroughly covered?
4. **Security invariant preserved?** — Does the code strictly adhere to all constraints in `ai/invariants.md`?
5. **External API verified?** — Are live SDK methods and endpoint schemas confirmed without invented fields?
6. **No secrets exposed?** — Are API keys, private keys, and authorization tokens absent from code, logs, and client bundles?
7. **No undocumented assumptions?** — Are all numerical units, decimal precisions, and defaults documented in `ai/`?
8. **Mainnet impact understood?** — Is on-chain rent exemption, compute budget, and transaction determinism accounted for?

---

## Reviews Directory

- [architecture-review.md](file:///home/seam/Desktop/project/equity-catalyst/ai/reviews/architecture-review.md) — Structural decoupling, async throughput, and domain model boundaries.
- [security-review.md](file:///home/seam/Desktop/project/equity-catalyst/ai/reviews/security-review.md) — Dual-line defense, signer isolation, input sanitation, and reentrancy safeguards.
- [integration-review.md](file:///home/seam/Desktop/project/equity-catalyst/ai/reviews/integration-review.md) — Pyth Pro Hermes, Meteora DBC SDK, and Jupiter v6 routing evaluations.
- [submission-review.md](file:///home/seam/Desktop/project/equity-catalyst/ai/reviews/submission-review.md) — Comprehensive hackathon criteria checklist and proof of execution.
