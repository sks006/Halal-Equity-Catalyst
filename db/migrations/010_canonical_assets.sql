-- 010_canonical_assets.sql: Canonical database-backed asset registry table
-- Stores source-of-truth tokenized equity identities rooted in mint address

CREATE TABLE IF NOT EXISTS canonical_assets (
    asset_id VARCHAR(64) PRIMARY KEY,
    symbol VARCHAR(16) NOT NULL,
    mint_address VARCHAR(44) NOT NULL,
    legal_issuer VARCHAR(128) NOT NULL,
    custodian VARCHAR(128) NOT NULL,
    underlying_asset_identifier VARCHAR(128) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    approval_status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    decimals SMALLINT NOT NULL DEFAULT 6,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_canonical_assets_mint UNIQUE (mint_address)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_canonical_assets_mint ON canonical_assets(mint_address);
CREATE INDEX IF NOT EXISTS idx_canonical_assets_symbol ON canonical_assets(symbol);
CREATE INDEX IF NOT EXISTS idx_canonical_assets_active_approved ON canonical_assets(is_active, approval_status);
CREATE INDEX IF NOT EXISTS idx_canonical_assets_status ON canonical_assets(approval_status);

CREATE TRIGGER update_canonical_assets_updated_at
BEFORE UPDATE ON canonical_assets
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
