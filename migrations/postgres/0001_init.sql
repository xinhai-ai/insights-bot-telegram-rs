-- Postgres initial schema for Telegram recap bot
CREATE TABLE IF NOT EXISTS chats (
    id BIGINT PRIMARY KEY,
    title TEXT NULL,
    username TEXT NULL,
    kind TEXT NULL,
    created_at BIGINT NULL,
    updated_at BIGINT NULL
);

CREATE TABLE IF NOT EXISTS chat_histories (
    id BIGSERIAL PRIMARY KEY,
    chat_id BIGINT NOT NULL,
    message_id BIGINT NOT NULL,
    from_id BIGINT NULL,
    from_full_name TEXT NULL,
    from_username TEXT NULL,
    kind TEXT NOT NULL,
    text TEXT NULL,
    media_url TEXT NULL,
    created_at BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chat_histories_chat_created ON chat_histories (chat_id, created_at);

-- Migration: Add from_full_name column if it doesn't exist (for existing databases)
ALTER TABLE chat_histories ADD COLUMN IF NOT EXISTS from_full_name TEXT NULL;

CREATE TABLE IF NOT EXISTS recap_configs (
    chat_id BIGINT PRIMARY KEY,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    auto_recap_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    last_recap_at BIGINT NULL,
    updated_at BIGINT NULL
);

CREATE TABLE IF NOT EXISTS recap_logs (
    id TEXT PRIMARY KEY,
    chat_id BIGINT NOT NULL,
    prompt TEXT NULL,
    recap_text TEXT NULL,
    model TEXT NULL,
    prompt_tokens INTEGER NULL,
    completion_tokens INTEGER NULL,
    created_at BIGINT NULL
);
CREATE INDEX IF NOT EXISTS idx_recap_logs_chat_created ON recap_logs (chat_id, created_at);
