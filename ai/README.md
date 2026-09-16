# AI Engineering Context

Project: Equity Catalyst

## Mission

Build an equity-aware liquidity and price-discovery platform for
tokenized/pre-IPO assets using:

- Pyth Pro
- Meteora DBC
- PreStocks
- Tessera
- Solana
- Anchor
- Rust/Axum/Tokio
- Next.js/TypeScript

## Mandatory reading order

Before modifying code, read:

1. [ai/README.md](file:///home/seam/Desktop/project/equity-catalyst/ai/README.md)
2. [ai/architecture.md](file:///home/seam/Desktop/project/equity-catalyst/ai/architecture.md)
3. [ai/module-map.md](file:///home/seam/Desktop/project/equity-catalyst/ai/module-map.md)
4. [ai/current-state.md](file:///home/seam/Desktop/project/equity-catalyst/ai/current-state.md)

Then read only the task-specific files.

## Standard Operating Procedure (SOP)

Every coding agent MUST follow this exact sequence:

```
┌─────────────────────────────┐
│ READ AI CONTEXT             │
│ README                      │
│ architecture                │
│ module-map                  │
│ current-state               │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ READ TASK                   │
│ phase-X                     │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ INSPECT ONLY RELEVANT CODE  │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ WRITE IMPLEMENTATION PLAN   │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ IMPLEMENT                   │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ TEST                        │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ REVIEW AGAINST INVARIANTS   │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ UPDATE CURRENT STATE        │
└─────────────────────────────┘
```

## Rules

- Never invent external API fields.
- Verify current SDK/API behavior before implementation.
- Never expose Pyth Pro API keys.
- Never put signing secrets in frontend code.
- Never bypass risk validation.
- Never let an LLM directly authorize financial transactions.
- Do not modify modules outside the task boundary without documenting why.
- Every implementation must include tests.
- Every task must finish with verification commands.
- Never claim mainnet functionality without a real verified transaction/address.
- Prefer existing abstractions over duplicated logic.

## Task workflow

READ
→ PLAN
→ IMPLEMENT
→ TEST
→ REVIEW
→ UPDATE CURRENT STATE

## Definition of Done

A task is not done until:

- code compiles
- tests pass
- integration behavior is verified where applicable
- documentation is updated
- [ai/current-state.md](file:///home/seam/Desktop/project/equity-catalyst/ai/current-state.md) is updated
- unresolved issues are recorded

## Standard Implementation Prompt

For any coding agent tackling a task, use the standard prompt template defined in [ai/agent-prompt.md](file:///home/seam/Desktop/project/equity-catalyst/ai/agent-prompt.md).

## The Most Important AI-Agent Rule

**Do not tell the agents:**
> *"Build Equity Catalyst."*

**Tell them:**
> *"Implement `ai/tasks/phase-01-pyth.md`."*

Then:
> *"Implement `ai/tasks/phase-03-meteora.md`."*

Then:
> *"Implement `ai/tasks/phase-04-equity-engine.md`."*

*This turns the AI from a general-purpose coder into a controlled team of specialized engineers.*

---

## Operating Model

```
                  ai/
                   │
        ┌──────────┼──────────┐
        ↓          ↓          ↓
 Architecture   Tasks     Invariants
        │          │          │
        └──────────┼──────────┘
                   ↓
              AI Agents
                   │
                   ↓
                Code
                   │
          ┌────────┴────────┐
          ↓                 ↓
        Tests             Review
          │                 │
          └────────┬────────┘
                   ↓
            Current State
                   │
                   ↓
              Next Agent
```


