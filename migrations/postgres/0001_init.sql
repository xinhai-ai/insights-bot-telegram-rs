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
    from_username TEXT NULL,
    kind TEXT NOT NULL,
    text TEXT NULL,
    media_url TEXT NULL,
    created_at BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chat_histories_chat_created ON chat_histories (chat_id, created_at);

CREATE TABLE IF NOT EXISTS recap_configs (
    chat_id BIGINT PRIMARY KEY,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    mode TEXT NULL,
    auto_recap_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    auto_recap_rates_per_day INTEGER NULL,
    last_recap_at BIGINT NULL,
    pinned_message_id BIGINT NULL,
    updated_at BIGINT NULL
);

CREATE TABLE IF NOT EXISTS recap_subscriptions (
    id TEXT PRIMARY KEY,
    chat_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    created_at BIGINT NULL
);
CREATE INDEX IF NOT EXISTS idx_recap_subscriptions_chat ON recap_subscriptions (chat_id);

CREATE TABLE IF NOT EXISTS recap_logs (
    id TEXT PRIMARY KEY,
    chat_id BIGINT NOT NULL,
    prompt TEXT NULL,
    recap_text TEXT NULL,
    model TEXT NULL,
    prompt_tokens INTEGER NULL,
    completion_tokens INTEGER NULL,
    feedback TEXT NULL,
    created_at BIGINT NULL
);
CREATE INDEX IF NOT EXISTS idx_recap_logs_chat_created ON recap_logs (chat_id, created_at);

CREATE TABLE IF NOT EXISTS forwarded_histories (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    from_chat_id BIGINT NULL,
    message_id BIGINT NULL,
    kind TEXT NOT NULL,
    text TEXT NULL,
    created_at BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_forwarded_histories_user_created ON forwarded_histories (user_id, created_at);
