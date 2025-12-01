use anyhow::Result;
use sqlx::AnyPool;

use super::models::{ChatHistory, MessageKind};

pub async fn insert_message(
    pool: &AnyPool,
    chat_id: i64,
    message_id: i64,
    from_id: Option<i64>,
    from_username: Option<String>,
    kind: MessageKind,
    text: Option<String>,
    media_url: Option<String>,
    created_at: i64,
) -> Result<()> {
    // Note: table schema expected to match fields; runtime will error if absent.
    sqlx::query(
        "INSERT INTO chat_histories (chat_id, message_id, from_id, from_username, kind, text, media_url, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(chat_id)
    .bind(message_id)
    .bind(from_id)
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
    let rows = sqlx::query_as::<_, ChatHistory>(
        "SELECT * FROM chat_histories WHERE chat_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(chat_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn recent_forwarded_messages(
    pool: &AnyPool,
    from_user_id: i64,
    limit: i64,
) -> Result<Vec<ChatHistory>> {
    let rows = sqlx::query_as::<_, ChatHistory>(
        "SELECT * FROM chat_histories WHERE from_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(from_user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
