# Dependency Map

## Allowed

```
shared
  ↓
api domain

api
  ↓
provider adapters
  ↓
external services

frontend
  ↓
API

Anchor
  ↓
Solana
```

## Forbidden

- `shared → api`
- `shared → frontend`
- `domain → provider-specific models`
- `frontend → Pyth secret`
- `agent → direct signer`
- `agent → direct Anchor bypass`
- `provider → business policy`
- `frontend → database`

---

### The Most Important Rule

```
Agent
  ↓
Risk Engine
  ↓
Execution Service
```

**Never**:

```
Agent
  ↓
Signer
```
