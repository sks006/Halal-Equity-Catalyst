-- 005_executions.sql: On-chain trade execution orders & transaction receipts

CREATE TABLE IF NOT EXISTS executions (
    execution_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vault_address VARCHAR(44) NOT NULL REFERENCES vaults(vault_address) ON DELETE CASCADE,
    event_id UUID REFERENCES events(event_id) ON DELETE SET NULL,
    action VARCHAR(32) NOT NULL,
    input_mint VARCHAR(44) NOT NULL,
    output_mint VARCHAR(44) NOT NULL,
    amount_in BIGINT NOT NULL,
    amount_out_expected BIGINT NOT NULL,
    amount_out_actual BIGINT,
    slippage_bps INTEGER NOT NULL,
    tx_signature VARCHAR(88),
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    error_message TEXT,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_executions_vault ON executions(vault_address);
CREATE INDEX IF NOT EXISTS idx_executions_event ON executions(event_id);
CREATE INDEX IF NOT EXISTS idx_executions_status ON executions(status);
CREATE INDEX IF NOT EXISTS idx_executions_tx ON executions(tx_signature);
