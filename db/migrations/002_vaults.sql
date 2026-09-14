-- 002_vaults.sql: On-chain vault states mirror table

CREATE TABLE IF NOT EXISTS vaults (
    vault_address VARCHAR(44) PRIMARY KEY,
    authority VARCHAR(44) NOT NULL,
    name VARCHAR(32) NOT NULL,
    symbol VARCHAR(12) NOT NULL,
    deposit_mint VARCHAR(44) NOT NULL,
    vault_token_account VARCHAR(44) NOT NULL,
    total_shares BIGINT NOT NULL DEFAULT 0,
    total_deposits BIGINT NOT NULL DEFAULT 0,
    is_paused BOOLEAN NOT NULL DEFAULT FALSE,
    bump SMALLINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_vaults_authority ON vaults(authority);
CREATE INDEX IF NOT EXISTS idx_vaults_symbol ON vaults(symbol);

CREATE TRIGGER update_vaults_updated_at
BEFORE UPDATE ON vaults
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
