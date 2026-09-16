# Known Issues & Integration Caveats

This document records active technical caveats, upstream library subtleties, and edge cases across Equity Catalyst components.

---

## 1. Meteora Dynamic Bonding Curve (DBC) SDK

### Monotonic SqrtPrice Progression
- **Caveat**: `buildCurveWithCustomSqrtPrices` and `createSqrtPrices` will fail with an assertion error if price points are not strictly monotonically increasing ($P_0 < P_1 < P_2 < P_3$).
- **Mitigation**: `buildEquityDiscoveryCurve` performs explicit pre-validation:
  ```typescript
  if (p0 <= 0 || p1 <= p0 || p2 <= p1 || p3 <= p2) {
    throw new Error(`Invalid regime price progression: P0=${p0}, P1=${p1}, P2=${p2}, P3=${p3}`);
  }
  ```

### SPL Token vs Token-2022
- **Caveat**: Backed Finance xStocks (e.g. `NVDAx`) are standard SPL Token mints with 8 decimals. However, certain quote mints or newer tokens utilize Token-2022 extensions.
- **Mitigation**: When interacting with Token-2022 mints, supply the required token badge and pass `TokenType.Token2022` to the SDK builder.

---

## 2. PostgreSQL Floating Point vs Numeric

### Tokio-Postgres Serialization
- **Caveat**: Rust `tokio-postgres` does not natively deserialize SQL `numeric` types into `f64` unless the `with-rust_decimal-1` feature and `rust_decimal` crate are utilized.
- **Mitigation**: Financial tables store `DOUBLE PRECISION` for floating values, and atomic integer units (`BIGINT` / `i64`) for on-chain lamports/shares.

---

## 3. Solana Devnet WebSocket Disconnects

### Transient TCP Drops
- **Caveat**: Public Solana Devnet WebSocket endpoints (`wss://api.devnet.solana.com`) frequently disconnect idle connections after 60-120 seconds.
- **Mitigation**: `EventListener` implements exponential backoff reconnection logic with periodic heartbeat ping frames.
