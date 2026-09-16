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
