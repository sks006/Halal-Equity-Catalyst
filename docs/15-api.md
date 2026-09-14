# 15. REST API Specification

## 1. Overview & Router Pipeline

The Equity Catalyst HTTP API is powered by Axum v0.7 on Tokio. It provides a RESTful interface for web clients, external keepers, and automated ingestion services.

* **Base URL**: `http://localhost:8080/api/v1` (or root path `/`)
* **Default Port**: `8080` (configured via `PORT` environment variable)
* **Middleware Pipeline**:
  * `tower_http::trace::TraceLayer`: Request/response structured tracing.
  * `tower_http::cors::CorsLayer`: Permissive CORS allowing multi-origin web dashboard access.
  * Axum State: Thread-safe `Arc<AppState>`.

---

## 2. Endpoint Specifications

### 2.1 System Health & Liveness

#### `GET /health`
* **Purpose**: Basic liveness probe for load balancers.
* **Auth**: Public.
* **Response**: `200 OK`
  ```json
  { "status": "ok" }
  ```

#### `GET /ready`
* **Purpose**: Readiness probe asserting PostgreSQL and Solana connectivity.
* **Response**: `200 OK`
  ```json
  { "status": "ready", "database": "connected", "solana": "connected" }
  ```

#### `GET /health/detailed` (also `/health/monitor`)
* **Purpose**: Deep telemetry check reporting connection pool health, RPC block height, and memory stats.
* **Response**: `200 OK`
  ```json
  {
    "status": "healthy",
    "timestamp": "2026-09-14T12:00:00Z",
    "uptime_seconds": 3600,
    "database": { "status": "up", "latency_ms": 2 },
    "redis": { "status": "up", "queue_length": 0 },
    "solana": { "status": "up", "cluster": "devnet", "slot": 284129480 }
  }
  ```

---

### 2.2 Oracle & Pricing Services

#### `GET /oracle/price/:symbol`
* **Purpose**: Fetches real-time price from Pyth Hermes API with standardized 6-decimal normalization.
* **Path Parameters**: `symbol` (e.g. `NVDA`, `MSFT`, `USDC`).
* **Response**: `200 OK`
  ```json
  {
    "symbol": "NVDA",
    "price_usd": 128.50,
    "raw_price": 128500000,
    "confidence_usd": 0.05,
    "publish_time": 1726315200
  }
  ```
* **Errors**: `404 Not Found` if feed ID not mapped in Pyth registry.

---

### 2.3 Quote & Risk Evaluation

#### `POST /quotes/evaluate`
* **Purpose**: Evaluates a potential swap quote through Jupiter v6 and the Risk Engine without executing.
* **Request Body**:
  ```json
  {
    "vault_address": "8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH",
    "input_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "output_mint": "NVDAxMintAddress11111111111111111111111111",
    "amount_in": 500000000,
    "slippage_bps": 50,
    "target_symbol": "NVDA"
  }
  ```
* **Validation**:
  * Vault must not be paused.
  * Policy must be active.
  * Price impact must not exceed `rebalance_threshold_bps`.
  * Post-trade exposure must not exceed `max_position_bps`.
* **Response**: `200 OK`
  ```json
  {
    "execution_id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d",
    "vault_address": "8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH",
    "input_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "output_mint": "NVDAxMintAddress11111111111111111111111111",
    "amount_in": 500000000,
    "expected_amount_out": 3891050,
    "min_amount_out": 3871594,
    "price_impact_bps": 12,
    "price_impact_pct": "0.12",
    "approved": true,
    "rejection_reason": null,
    "evaluated_exposure_bps": 3250,
    "is_dry_run": true,
    "evaluated_at": "2026-09-14T12:00:01Z"
  }
  ```
* **Side Effects**: Inserts a record in `executions` with `status = 'QUOTE_ONLY'` (or `'REJECTED'`).

---

### 2.4 Vault Management

#### `GET /vaults`
* **Purpose**: Lists all registered vaults.
* **Response**: `200 OK` (Array of `VaultModel`).

#### `POST /vaults`
* **Purpose**: Registers a new vault record in the database mirror.
* **Request Body**:
  ```json
  {
    "vault_address": "8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH",
    "authority": "Auth111111111111111111111111111111111111111",
    "name": "US Tech Growth Vault",
    "symbol": "USTECH",
    "deposit_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "vault_token_account": "VaultAta111111111111111111111111111111111",
    "bump": 255
  }
  ```
* **Response**: `201 Created`.

#### `GET /vaults/:address`
* **Purpose**: Retrieves a specific vault's state.

---

### 2.5 Policy Management

#### `GET /policies/:vault_address` (or `/vaults/:address/policy`)
* **Purpose**: Retrieves risk policy parameters for a vault.
* **Response**: `200 OK`
  ```json
  {
    "policy_address": "PolicyPDA11111111111111111111111111111111",
    "vault_address": "8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH",
    "authority": "Auth111111111111111111111111111111111111111",
    "max_ltv_bps": 5000,
    "max_position_bps": 3500,
    "stop_loss_bps": 500,
    "take_profit_bps": 1500,
    "rebalance_threshold_bps": 200,
    "is_active": true
  }
  ```

#### `POST /policies` (or `PUT /vaults/:address/policy`)
* **Purpose**: Creates or updates policy risk parameters.

---

### 2.6 Event Ingestion

#### `POST /events`
* **Purpose**: Ingests external market or earnings events to trigger policy evaluation.
* **Request Body**:
  ```json
  {
    "vault_address": "8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH",
    "event_type": "earnings_beat",
    "source": "Bloomberg Terminal",
    "sentiment_score": 0.85,
    "payload": {
      "symbol": "NVDA",
      "eps_actual": 0.68,
      "eps_estimate": 0.64,
      "surprise_pct": 8.0
    }
  }
  ```
* **Response**: `201 Created` with generated `event_id` (UUIDv4).
* **Side Effects**: Inserts row with `status = 'PENDING'`, queues for `PolicyWorker`.

#### `GET /events/pending`
* **Purpose**: Lists pending events awaiting processing.

---

### 2.7 Execution Audits

#### `GET /executions`
* **Purpose**: Lists all historical execution decisions and quotes.

#### `GET /vaults/:address/executions`
* **Purpose**: Filters execution history by vault.
