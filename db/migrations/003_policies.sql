-- 003_policies.sql: Policy engine risk parameters mirror table

CREATE TABLE IF NOT EXISTS policies (
    policy_address VARCHAR(44) PRIMARY KEY,
    vault_address VARCHAR(44) NOT NULL REFERENCES vaults(vault_address) ON DELETE CASCADE,
    authority VARCHAR(44) NOT NULL,
    max_ltv_bps INTEGER NOT NULL,
    max_position_bps INTEGER NOT NULL,
    stop_loss_bps INTEGER NOT NULL,
    take_profit_bps INTEGER NOT NULL,
    rebalance_threshold_bps INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    bump SMALLINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_policies_vault UNIQUE (vault_address)
);

CREATE INDEX IF NOT EXISTS idx_policies_vault ON policies(vault_address);

CREATE TRIGGER update_policies_updated_at
BEFORE UPDATE ON policies
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
