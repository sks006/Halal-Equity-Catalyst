-- 009_dead_letters.sql: Dead-letter records for unprocessable or rejected events

CREATE TABLE IF NOT EXISTS dead_letters (
    dead_letter_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID,
    failure_reason TEXT NOT NULL,
    payload_reference JSONB NOT NULL DEFAULT '{}'::jsonb,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dead_letters_event_id ON dead_letters(event_id);
CREATE INDEX IF NOT EXISTS idx_dead_letters_created_at ON dead_letters(created_at);
