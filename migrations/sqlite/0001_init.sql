-- SQLite initial schema for Telegram recap bot
CREATE TABLE IF NOT EXISTS chats (
    id INTEGER PRIMARY KEY,
    title TEXT,
    username TEXT,
    kind TEXT,
    created_at INTEGER,
    updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS chat_histories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_id INTEGER NOT NULL,
    message_id INTEGER NOT NULL,
    from_id INTEGER,
    from_full_name TEXT,
    from_username TEXT,
    kind TEXT NOT NULL,
    text TEXT,
    media_url TEXT,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chat_histories_chat_created ON chat_histories (chat_id, created_at);

-- Migration: Add from_full_name column if it doesn't exist (for existing databases)
-- SQLite doesn't have IF NOT EXISTS for columns, so we use a workaround
-- This will fail silently if the column already exists

CREATE TABLE IF NOT EXISTS recap_configs (
    chat_id INTEGER PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 1,
    auto_recap_enabled INTEGER NOT NULL DEFAULT 0,
    last_recap_at INTEGER,
    updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS recap_logs (
    id TEXT PRIMARY KEY,
    chat_id INTEGER NOT NULL,
    prompt TEXT,
    recap_text TEXT,
    model TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    created_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_recap_logs_chat_created ON recap_logs (chat_id, created_at);
