use anyhow::Result;
use sqlx::AnyPool;
use tracing::info;

/// Migrate all data from old_chat_id to new_chat_id in a single transaction.
/// Used when Telegram upgrades a group to a supergroup.
pub async fn migrate_chat_data(pool: &AnyPool, old_chat_id: i64, new_chat_id: i64) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Update chat_histories
    sqlx::query("UPDATE chat_histories SET chat_id = $1 WHERE chat_id = $2")
        .bind(new_chat_id)
        .bind(old_chat_id)
        .execute(&mut *tx)
        .await?;

    // Migrate recap_configs: check for conflict first
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT chat_id FROM recap_configs WHERE chat_id = $1 LIMIT 1",
    )
    .bind(new_chat_id)
    .fetch_optional(&mut *tx)
    .await?;

    if existing.is_none() {
        // No conflict, safe to update
        sqlx::query("UPDATE recap_configs SET chat_id = $1, updated_at = $2 WHERE chat_id = $3")
            .bind(new_chat_id)
            .bind(chrono::Utc::now().timestamp())
            .bind(old_chat_id)
            .execute(&mut *tx)
            .await?;
    } else {
        // Conflict: delete old config (new one takes precedence)
        sqlx::query("DELETE FROM recap_configs WHERE chat_id = $1")
            .bind(old_chat_id)
            .execute(&mut *tx)
            .await?;
    }

    // Migrate chats table: check if new exists, copy old to new, then delete old.
    let existing_chat: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM chats WHERE id = $1 LIMIT 1",
    )
    .bind(new_chat_id)
    .fetch_optional(&mut *tx)
    .await?;

    if existing_chat.is_none() {
        sqlx::query(
            "INSERT INTO chats (id, title, username, kind, created_at, updated_at)
             SELECT $1, title, username, kind, created_at, $2
             FROM chats WHERE id = $3",
        )
        .bind(new_chat_id)
        .bind(chrono::Utc::now().timestamp())
        .bind(old_chat_id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("DELETE FROM chats WHERE id = $1")
        .bind(old_chat_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    info!(
        old_chat_id = old_chat_id,
        new_chat_id = new_chat_id,
        "chat migration completed"
    );
    Ok(())
}
