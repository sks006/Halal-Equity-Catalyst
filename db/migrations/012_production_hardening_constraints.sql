-- 012_production_hardening_constraints.sql: Production data integrity constraints and idempotency indexes

-- 1. Executions table positive financial value & bounds constraints
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_executions_positive_amount_in'
    ) THEN
        ALTER TABLE executions
            ADD CONSTRAINT chk_executions_positive_amount_in
            CHECK (amount_in > 0);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_executions_non_negative_expected_out'
    ) THEN
        ALTER TABLE executions
            ADD CONSTRAINT chk_executions_non_negative_expected_out
            CHECK (amount_out_expected >= 0);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_executions_valid_slippage'
    ) THEN
        ALTER TABLE executions
            ADD CONSTRAINT chk_executions_valid_slippage
            CHECK (slippage_bps >= 0 AND slippage_bps <= 10000);
    END IF;
END $$;

-- 2. Partial unique index to enforce transaction hash replay protection
CREATE UNIQUE INDEX IF NOT EXISTS uq_executions_confirmed_tx
    ON executions(tx_signature)
    WHERE tx_signature IS NOT NULL;

-- 3. Partial unique index to prevent duplicate executions for the same policy decision
CREATE UNIQUE INDEX IF NOT EXISTS uq_executions_active_policy_decision
    ON executions(policy_decision_id)
    WHERE policy_decision_id IS NOT NULL AND status IN ('CONFIRMED', 'PENDING');

-- 4. Policies table risk bounds constraints
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_policies_max_position_bps'
    ) THEN
        ALTER TABLE policies
            ADD CONSTRAINT chk_policies_max_position_bps
            CHECK (max_position_bps >= 0 AND max_position_bps <= 10000);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_policies_stop_loss_bps'
    ) THEN
        ALTER TABLE policies
            ADD CONSTRAINT chk_policies_stop_loss_bps
            CHECK (stop_loss_bps >= 0 AND stop_loss_bps <= 10000);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_policies_rebalance_bps'
    ) THEN
        ALTER TABLE policies
            ADD CONSTRAINT chk_policies_rebalance_bps
            CHECK (rebalance_threshold_bps >= 0 AND rebalance_threshold_bps <= 10000);
    END IF;
END $$;
