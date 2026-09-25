-- 006_execution_records.sql: Add quote_id, policy_decision_id, and amount_out_min to executions table
ALTER TABLE executions ADD COLUMN IF NOT EXISTS quote_id VARCHAR(64);
ALTER TABLE executions ADD COLUMN IF NOT EXISTS policy_decision_id UUID;
ALTER TABLE executions ADD COLUMN IF NOT EXISTS amount_out_min BIGINT;

CREATE INDEX IF NOT EXISTS idx_executions_quote ON executions(quote_id);
CREATE INDEX IF NOT EXISTS idx_executions_policy_decision ON executions(policy_decision_id);
