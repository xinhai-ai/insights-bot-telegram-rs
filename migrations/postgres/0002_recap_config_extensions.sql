-- Add auto-recap frequency, pin toggle, and last pinned message tracking to recap_configs
ALTER TABLE recap_configs ADD COLUMN IF NOT EXISTS auto_recap_rates_per_day INTEGER NOT NULL DEFAULT 4;
ALTER TABLE recap_configs ADD COLUMN IF NOT EXISTS pin_auto_recap_message BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE recap_configs ADD COLUMN IF NOT EXISTS last_pinned_message_id BIGINT NULL;
