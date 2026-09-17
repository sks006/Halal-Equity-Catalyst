-- 008_remove_ltv_spot_policy.sql: Replace LTV with minimum cash reserve for spot-only Shariah compliance

ALTER TABLE policies DROP COLUMN IF EXISTS max_ltv_bps;
ALTER TABLE policies ADD COLUMN IF NOT EXISTS min_cash_bps INTEGER NOT NULL DEFAULT 1000;
