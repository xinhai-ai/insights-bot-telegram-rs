use anyhow::Result;
use sqlx::AnyPool;

use super::models::{ChatHistory, MessageKind};

#[allow(clippy::too_many_arguments)]
pub async fn insert_message(
    pool: &AnyPool,
    chat_id: i64,
    message_id: i64,
    from_id: Option<i64>,
    from_full_name: Option<String>,
    from_username: Option<String>,
    kind: MessageKind,
    text: Option<String>,
    media_url: Option<String>,
    created_at: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO chat_histories (chat_id, message_id, from_id, from_full_name, from_username, kind, text, media_url, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(chat_id)
    .bind(message_id)
    .bind(from_id)
    .bind(from_full_name)
    .bind(from_username)
    .bind(kind.as_str())
    .bind(text)
    .bind(media_url)
    .bind(created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn recent_messages(pool: &AnyPool, chat_id: i64, limit: i64) -> Result<Vec<ChatHistory>> {
    // Use explicit column selection with COALESCE to handle SQLx Any driver NULL issues.
    let rows = sqlx::query_as::<_, ChatHistory>(
        "SELECT id, chat_id, message_id, from_id,
                COALESCE(from_full_name, '') as from_full_name,
                COALESCE(from_username, '') as from_username,
                kind,
                COALESCE(text, '') as text,
                COALESCE(media_url, '') as media_url,
                created_at
         FROM chat_histories WHERE chat_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(chat_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn is_recap_enabled(pool: &AnyPool, chat_id: i64) -> Result<bool> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT CASE WHEN enabled THEN 1 ELSE 0 END
         FROM recap_configs
         WHERE chat_id = $1
         LIMIT 1",
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(enabled,)| enabled != 0).unwrap_or(true))
}

/// Find chat messages within the specified time duration (hours) before now.
pub async fn messages_since_hours(
    pool: &AnyPool,
    chat_id: i64,
    hours: i64,
) -> Result<Vec<ChatHistory>> {
    let since_timestamp = chrono::Utc::now().timestamp() - (hours * 3600);
    let rows = sqlx::query_as::<_, ChatHistory>(
        "SELECT id, chat_id, message_id, from_id,
                COALESCE(from_full_name, '') as from_full_name,
                COALESCE(from_username, '') as from_username,
                kind,
                COALESCE(text, '') as text,
                COALESCE(media_url, '') as media_url,
                created_at
         FROM chat_histories
         WHERE chat_id = $1 AND created_at >= $2
         ORDER BY created_at ASC",
    )
    .bind(chat_id)
    .bind(since_timestamp)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Count messages in a chat within the specified time duration.
#[allow(dead_code)]
pub async fn count_messages_since_hours(pool: &AnyPool, chat_id: i64, hours: i64) -> Result<i64> {
    let since_timestamp = chrono::Utc::now().timestamp() - (hours * 3600);
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM chat_histories WHERE chat_id = $1 AND created_at >= $2",
    )
    .bind(chat_id)
    .bind(since_timestamp)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Update the text of an existing message (for edited-message sync).
pub async fn update_message_text(
    pool: &AnyPool,
    chat_id: i64,
    message_id: i64,
    new_text: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE chat_histories SET text = $1 WHERE chat_id = $2 AND message_id = $3",
    )
    .bind(new_text)
    .bind(chat_id)
    .bind(message_id)
    .execute(pool)
    .await?;
    Ok(())
}
