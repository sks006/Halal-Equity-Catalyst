-- 004_events.sql: External market, oracle, and news events table

CREATE TABLE IF NOT EXISTS events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vault_address VARCHAR(44) REFERENCES vaults(vault_address) ON DELETE CASCADE,
    event_type VARCHAR(64) NOT NULL,
    source VARCHAR(64) NOT NULL,
    sentiment_score DOUBLE PRECISION,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_events_vault ON events(vault_address);
CREATE INDEX IF NOT EXISTS idx_events_status ON events(status);
CREATE INDEX IF NOT EXISTS idx_events_detected_at ON events(detected_at DESC);
