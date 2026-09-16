-- 007_dbc_pools.sql: Launched Meteora DBC pools for real tokenized equities

CREATE TABLE IF NOT EXISTS dbc_pools (
    pool_address VARCHAR(44) PRIMARY KEY,
    config_address VARCHAR(44) NOT NULL,
    base_mint VARCHAR(44) NOT NULL,
    quote_mint VARCHAR(44) NOT NULL,
    token_name VARCHAR(64) NOT NULL,
    token_symbol VARCHAR(16) NOT NULL,
    tx_signature VARCHAR(88) NOT NULL,
    creator VARCHAR(44) NOT NULL,
    initial_price_usd DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    current_price_usd DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    curve_progress_pct DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    is_migrated BOOLEAN NOT NULL DEFAULT FALSE,
    creation_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dbc_pools_config ON dbc_pools(config_address);
CREATE INDEX IF NOT EXISTS idx_dbc_pools_base_mint ON dbc_pools(base_mint);
CREATE INDEX IF NOT EXISTS idx_dbc_pools_quote_mint ON dbc_pools(quote_mint);
CREATE INDEX IF NOT EXISTS idx_dbc_pools_symbol ON dbc_pools(token_symbol);
CREATE INDEX IF NOT EXISTS idx_dbc_pools_creator ON dbc_pools(creator);

CREATE TRIGGER update_dbc_pools_updated_at
BEFORE UPDATE ON dbc_pools
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
