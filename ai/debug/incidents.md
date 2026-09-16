# Incident History & Postmortems

This document logs significant development and staging incidents, their root causes, and subsequent architectural fixes.

---

## INC-001: Tokio-Postgres Type Mismatch on Pool Precision

- **Date**: 2026-09-16
- **Component**: `apps/api/src/repositories/dbc_pool_repository.rs` & `db/migrations/007_dbc_pools.sql`
- **Impact**: Database query error when querying numeric columns using standard `f64` types.
- **Root Cause**: PostgreSQL `NUMERIC(18,6)` maps to `rust_decimal::Decimal` in tokio-postgres, rather than primitive `f64`.
- **Resolution**: Altered `initial_price_usd`, `current_price_usd`, and `curve_progress_pct` columns to `DOUBLE PRECISION` to match the established pattern in `portfolios` table.

---

## INC-002: Cargo Check Target Name Discrepancy

- **Date**: 2026-09-16
- **Component**: `apps/api/Cargo.toml`
- **Impact**: Invoking `cargo check --bin api` resulted in `no bin target named api`.
- **Root Cause**: Package binary was declared as `equity-catalyst-api`.
- **Resolution**: Updated standard execution commands to use `cargo check --bin equity-catalyst-api`.
