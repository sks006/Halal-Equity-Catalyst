-- 011_asset_market_data.sql: Explicit database-backed mapping between approved assets and Pyth price feeds

CREATE TABLE IF NOT EXISTS asset_market_data (
    mapping_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id VARCHAR(64) NOT NULL REFERENCES canonical_assets(asset_id) ON DELETE CASCADE,
    pyth_feed_id VARCHAR(64) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- At most one active mapping per canonical asset
CREATE UNIQUE INDEX IF NOT EXISTS idx_asset_market_data_active_asset
    ON asset_market_data(asset_id)
    WHERE is_active = TRUE;

-- Pyth feed identity should not accidentally map to multiple active assets
CREATE UNIQUE INDEX IF NOT EXISTS idx_asset_market_data_active_feed
    ON asset_market_data(pyth_feed_id)
    WHERE is_active = TRUE;

CREATE INDEX IF NOT EXISTS idx_asset_market_data_asset ON asset_market_data(asset_id);
CREATE INDEX IF NOT EXISTS idx_asset_market_data_feed ON asset_market_data(pyth_feed_id);
CREATE INDEX IF NOT EXISTS idx_asset_market_data_is_active ON asset_market_data(is_active);

CREATE TRIGGER update_asset_market_data_updated_at
BEFORE UPDATE ON asset_market_data
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
