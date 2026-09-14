# 20. Deployment Architecture & Infrastructure Operations

## 1. Multi-Tier Deployment Topology

Equity Catalyst supports three distinct deployment targets:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            DEPLOYMENT TARGETS                               │
│                                                                             │
│  [Local Development]                                                        │
│    ├── Local Solana Test Validator (`solana-test-validator`)                │
│    ├── Docker Compose: PostgreSQL (Port 5432) & Redis (Port 6379)           │
│    ├── Axum API (`cargo run -p equity-catalyst-api`)                        │
│    └── Next.js Web Dashboard (`pnpm --filter web dev` on Port 3000)         │
│                                                                             │
│  [Solana Devnet]                                                            │
│    ├── On-Chain Program deployed to Devnet: 8NhtqxR1mwq7a3HTUtcGABNZ...     │
│    ├── Managed PostgreSQL (e.g. AWS RDS / Supabase)                         │
│    ├── Managed Redis (e.g. Upstash / AWS ElastiCache)                       │
│    └── Backend containerized via Docker on ECS / Fly.io                     │
│                                                                             │
│  [Production Target (Planned)]                                              │
│    ├── Solana Mainnet Program with Multisig Upgrade Authority (Squads)      │
│    ├── Cloud HSM / Turnkey Isolated Keypair Signer                          │
│    ├── High-Throughput Private RPC (Triton / Helius / Jito Block Engine)    │
│    └── High-Availability PostgreSQL with read-replicas                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Environment Configuration Matrix

Configuration is managed via environment variables defined in `.env`:

| Variable | Default (Local Dev) | Production Target | Description |
|---|---|---|---|
| `PORT` | `8080` | `8080` | Axum HTTP server port |
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/equity_catalyst` | Managed RDS connection string | PostgreSQL database connection string |
| `REDIS_URL` | `redis://localhost:6379` | Managed Redis TLS endpoint | Redis queue endpoint |
| `SOLANA_RPC_URL`| `http://127.0.0.1:8899` or `https://api.devnet.solana.com` | Dedicated Private RPC (Helius/Triton) | Solana JSON-RPC HTTP endpoint |
| `SOLANA_WS_URL` | `ws://127.0.0.1:8900` or `wss://api.devnet.solana.com` | Dedicated Private WSS | Solana WebSocket log subscription URL |
| `PROGRAM_ID` | `8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH` | Verified Mainnet Program ID | Deployed Anchor Program ID |
| `EXECUTION_SIGNER_KEY_PATH` | `~/.config/solana/id.json` | Path to secured key or HSM | Keypair path for execution signing |
| `RUST_LOG` | `info,equity_catalyst_api=debug` | `info` | Tracing filter level |

---

## 3. Step-by-Step Local Deployment Setup

### Step 1: Start PostgreSQL and Redis
Launch local persistence containers using Docker:
```bash
docker run -d --name equity-postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=equity_catalyst \
  -p 5432:5432 postgres:16-alpine

docker run -d --name equity-redis \
  -p 6379:6379 redis:7-alpine
```

### Step 2: Run Database Migrations
Apply schema migrations in sequence from `db/migrations/`:
```bash
for file in db/migrations/*.sql; do
  psql postgres://postgres:postgres@localhost:5432/equity_catalyst -f "$file"
done
```

### Step 3: Build & Deploy Anchor Program (Devnet / Local)
```bash
# Build the binary
anchor build

# Deploy to Devnet
solana config set --url devnet
solana airdrop 2 # If needed
anchor deploy --program-name equity_vault
```

### Step 4: Launch Backend API and Background Workers
```bash
cargo run -p equity-catalyst-api
```

### Step 5: Launch Next.js Web Dashboard
```bash
pnpm --filter web install
pnpm --filter web dev
```
Open `http://localhost:3000` in your browser.
