-- 006_portfolios.sql: Vault asset portfolio breakdown & weights table

CREATE TABLE IF NOT EXISTS portfolios (
    portfolio_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vault_address VARCHAR(44) NOT NULL REFERENCES vaults(vault_address) ON DELETE CASCADE,
    asset_symbol VARCHAR(16) NOT NULL,
    asset_mint VARCHAR(44) NOT NULL,
    amount BIGINT NOT NULL DEFAULT 0,
    entry_price_usd DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    current_price_usd DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    current_value_usd DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    target_weight_bps INTEGER NOT NULL DEFAULT 0,
    current_weight_bps INTEGER NOT NULL DEFAULT 0,
    last_rebalanced_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_portfolio_vault_asset UNIQUE (vault_address, asset_symbol)
);

CREATE INDEX IF NOT EXISTS idx_portfolios_vault ON portfolios(vault_address);
CREATE INDEX IF NOT EXISTS idx_portfolios_asset ON portfolios(asset_symbol);

CREATE TRIGGER update_portfolios_updated_at
BEFORE UPDATE ON portfolios
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
