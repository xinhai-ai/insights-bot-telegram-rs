-- Add auto-recap frequency, pin toggle, and last pinned message tracking to recap_configs
-- SQLite: use INTEGER for boolean (0/1) and omit IF NOT EXISTS (not supported for columns)
ALTER TABLE recap_configs ADD COLUMN auto_recap_rates_per_day INTEGER NOT NULL DEFAULT 4;
ALTER TABLE recap_configs ADD COLUMN pin_auto_recap_message INTEGER NOT NULL DEFAULT 0;
ALTER TABLE recap_configs ADD COLUMN last_pinned_message_id INTEGER;
