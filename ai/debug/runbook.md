# Operational Runbook

This runbook provides step-by-step procedures for operating, restarting, and verifying the Equity Catalyst infrastructure.

---

## 1. Container Infrastructure

### Start PostgreSQL 17 & Redis 7
```bash
docker run -d \
  --name equity-postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=equity_catalyst \
  -p 5432:5432 \
  postgres:17

docker run -d \
  --name equity-redis \
  -p 6379:6379 \
  redis:7
```

### Restart Existing Containers
```bash
docker restart equity-postgres equity-redis
```

### Check Container Status
```bash
docker ps --filter "name=equity-"
```

---

## 2. Database Migrations

### Apply Migrations
```bash
for file in db/migrations/*.sql; do
  echo "Applying $file..."
  docker exec -i equity-postgres psql -U postgres -d equity_catalyst < "$file"
done
```

### Inspect Database Schema
```bash
docker exec -it equity-postgres psql -U postgres -d equity_catalyst -c "\dt"
```

---

## 3. Backend API Service

### Build & Run
```bash
cd apps/api
cargo check --bin equity-catalyst-api
cargo run --bin equity-catalyst-api
```

### Health Verification
```bash
curl -s http://localhost:8080/health | jq .
curl -s http://localhost:8080/health/detailed | jq .
```

---

## 4. Frontend Web Dashboard

### Install & Run
```bash
cd apps/web
pnpm install
pnpm dev
```
Navigate to `http://localhost:3000`.

---

## 5. Administrative Authentication & Key Rotation

### Admin API Key Configuration
Administrative endpoints (`/vaults`, `/policies`, `/events`, `/dbc/*`) require the `x-admin-key` header or `Authorization: Bearer <key>`.
Set the key via the environment variable:
```bash
export ADMIN_API_KEY="your-high-entropy-random-secret"
```

### Rotating the Admin Key
1. Generate a new secret:
   ```bash
   openssl rand -hex 32
   ```
2. Update the environment variable in the deployment environment (`.env` or secret manager).
3. Restart the API service; existing sessions are stateless and immediately require the new key.
4. Verify using an authenticated request:
   ```bash
   curl -s -H "x-admin-key: $ADMIN_API_KEY" http://localhost:4000/api/v1/policies
   ```

---

## 6. Rate Limiting Management

### Configuration
Adjust the global rate limit via `RATE_LIMIT_REQUESTS_PER_MINUTE` (default: 120 requests/minute).
When rate-limited, clients receive `HTTP 429 Too Many Requests`:
```json
{
  "error": {
    "code": "TOO_MANY_REQUESTS",
    "message": "Rate limit exceeded. Please retry after window resets.",
    "timestamp": "2026-09-18T03:00:00Z"
  }
}
```

### Bypassing / Recovering from Rate Limits
1. The sliding window automatically prunes stale requests after 60 seconds.
2. In emergency operational scenarios, restart the API instance to flush in-memory sliding window counters.

---

## 7. Emergency Pause Procedures

### Halting All Trading on a Vault
If anomalous market or oracle conditions occur, pause the vault immediately:
1. Trigger vault pause via administrative endpoint or Anchor CLI:
   ```bash
   curl -X POST http://localhost:4000/api/v1/vaults/<VAULT_ADDRESS>/pause \
     -H "x-admin-key: $ADMIN_API_KEY"
   ```
2. Verify that all in-flight and pending executions fail closed:
   - Any transaction in pre-execution or pre-signing check will abort with `Vault is paused`.
   - Signer is never invoked for paused vaults.

---

## 8. Dead-Letter Queue & Incident Post-Mortem

### Inspecting Dead Letters
Failed, unprocessable, or rejected events are stored in the `dead_letters` table:
```bash
PGPASSWORD=postgres psql -h localhost -p 5432 -U postgres -d equity_catalyst -c "
  SELECT dead_letter_id, event_id, failure_reason, retry_count, created_at
  FROM dead_letters
  ORDER BY created_at DESC
  LIMIT 10;
"
```

### Reprocessing / Resolving Poison Pills
1. Retrieve full payload:
   ```bash
   PGPASSWORD=postgres psql -h localhost -p 5432 -U postgres -d equity_catalyst -c "
     SELECT payload_reference FROM dead_letters WHERE dead_letter_id = '<ID>';
   "
   ```
2. Diagnose root cause (e.g. malformed schema, prohibited sector, or network timeout).
3. If valid, re-submit event with corrected fields via the administrative `/events` endpoint.
