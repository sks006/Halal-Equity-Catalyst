# 19. External Integrations & Infrastructure

## 1. External Infrastructure Landscape

Equity Catalyst composes existing high-performance Solana protocols rather than rebuilding proprietary liquidity, oracle, or exchange infrastructure:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       EXTERNAL INTEGRATION ECOSYSTEM                        │
│                                                                             │
│   [Solana Cluster]       ──► RPC / WebSocket (`integrations/solana`)        │
│                              • Account state, SPL balances, Tx confirmation │
│                                                                             │
│   [Pyth Network]         ──► Hermes HTTP API (`integrations/pyth`)          │
│                              • Low-latency equity & crypto price feeds      │
│                                                                             │
│   [Jupiter DEX]          ──► v6 Quote & Swap API (`integrations/jupiter`)    │
│                              • Best-route DEX liquidity & price impact      │
│                                                                             │
│   [Tokenized Equities]   ──► Generic SPL Mints (xStocks/Ondo: PLANNED)      │
│                              • Fractional equity shares wrapped as SPL      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Integration Deep-Dive Specifications

### 2.1 Solana RPC & WebSocket (`integrations/solana`) `[IMPLEMENTED]`
* **Purpose**: Fetches account state, subscribes to on-chain program logs, and submits Anchor transactions.
* **Data Consumed**: Account binary data, slot height, transaction status, WebSocket log notifications.
* **Data Produced**: Serialized signed transactions, deserialized Anchor account models.
* **Trust Assumptions**: Assumes RPC node correctly validates cluster consensus and provides un-skewed account data.
* **Failure Modes**: Rate limiting (HTTP 429), WebSocket connection drops, expired recent blockhashes.
* **Fallback Mechanisms**:
  * Exponential backoff retry logic on RPC submission.
  * Automatic WebSocket reconnect loop.
  * Read-only safety lock (`allow_transactions: false` by default).

---

### 2.2 Pyth Network Hermes Oracle (`integrations/pyth`) `[IMPLEMENTED]`
* **Purpose**: Ingests real-time prices for equities and crypto assets via Pyth Hermes REST endpoints (`https://hermes.pyth.network`).
* **Data Consumed**: JSON price updates containing `price`, `conf`, `expo`, and `publish_time`.
* **Data Produced**: `NormalizedPrice` (standardized to 6 decimal places).
* **Trust Assumptions**: Trust in Pyth publisher network aggregate prices.
* **Failure Modes**: Network timeout, stale feeds, extreme confidence interval widening.
* **Mitigation & Fallback**:
  * Rejects prices where `conf > 0.02 * price` ($2\%$ confidence band).
  * Rejects invalid exponents ($expo \notin [-12, 12]$).
  * Built-in `new_mock()` mode for deterministic offline testing.

---

### 2.3 Jupiter v6 DEX Aggregator (`integrations/jupiter`) `[IMPLEMENTED]`
* **Purpose**: Fetches competitive swap routes, price impact metrics, and builds swap transactions across Solana AMMs.
* **Data Consumed**: `QuoteResponse` (`outAmount`, `priceImpactPct`, `otherAmountThreshold`).
* **Data Produced**: `SwapResponse` containing base64-encoded transaction instructions.
* **Trust Assumptions**: Assumes Jupiter routing algorithm finds optimal execution paths and calculates accurate price impact.
* **Failure Modes**: API unavailability, insufficient liquidity in token pools, routing slippage.
* **Mitigation & Fallback**:
  * `parse_price_impact_bps` strictly clamps impact and rejects quotes breaching policy limits.
  * Built-in `new_mock()` mode providing deterministic in-memory quotes.

---

### 2.4 Tokenized Equity Providers (xStocks, Ondo) `[PLANNED / NOT IMPLEMENTED]`
* **Current Status**:
  * The directories `integrations/xstocks/` and `integrations/ondo/` are currently **empty directories**.
  * No custom SDK or proprietary provider API is implemented in the repository.
* **Current Operational Workaround**:
  * The system treats tokenized equities as standard **SPL Token Mints** (e.g. `NVDAx`, `MSFTx`).
  * Any token compliant with the SPL Token Program standard can be held by the vault, quoted via Jupiter, and priced via Pyth.
* **Target Integration Plan**:
  * Direct integration with Ondo USDY tokenized treasury redemption APIs.
  * Direct integration with Backed Finance / xStocks primary issuance and mint/burn endpoints.
