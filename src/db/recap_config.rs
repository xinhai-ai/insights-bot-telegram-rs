use anyhow::Result;
use sqlx::{AnyPool, Row};

use super::models::RecapConfig;

pub async fn upsert_recap_config(pool: &AnyPool, cfg: &RecapConfig) -> Result<()> {
    sqlx::query(
        "INSERT INTO recap_configs (chat_id, enabled, auto_recap_enabled, last_recap_at, updated_at)
         VALUES ($1,$2,$3,$4,$5)
         ON CONFLICT (chat_id) DO UPDATE SET
           enabled = EXCLUDED.enabled,
           auto_recap_enabled = EXCLUDED.auto_recap_enabled,
           last_recap_at = EXCLUDED.last_recap_at,
           updated_at = EXCLUDED.updated_at",
    )
    .bind(cfg.chat_id)
    .bind(cfg.enabled)
    .bind(cfg.auto_recap_enabled)
    .bind(cfg.last_recap_at)
    .bind(cfg.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_recap_config(pool: &AnyPool, chat_id: i64) -> Result<Option<RecapConfig>> {
    let row = sqlx::query(
        "SELECT chat_id,
                CASE WHEN enabled THEN 1 ELSE 0 END AS enabled_flag,
                CASE WHEN auto_recap_enabled THEN 1 ELSE 0 END AS auto_recap_enabled_flag,
                last_recap_at,
                updated_at
         FROM recap_configs
         WHERE chat_id = $1
         LIMIT 1",
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(map_recap_config_row))
}

pub async fn get_or_create_recap_config(pool: &AnyPool, chat_id: i64) -> Result<RecapConfig> {
    if let Some(cfg) = get_recap_config(pool, chat_id).await? {
        return Ok(cfg);
    }
    let default = RecapConfig {
        chat_id,
        enabled: true,
        auto_recap_enabled: false,
        last_recap_at: None,
        updated_at: None,
    };
    upsert_recap_config(pool, &default).await?;
    Ok(default)
}

pub async fn list_due_for_auto_recap(pool: &AnyPool, due_before: i64) -> Result<Vec<RecapConfig>> {
    let rows = sqlx::query(
        "SELECT chat_id,
                CASE WHEN enabled THEN 1 ELSE 0 END AS enabled_flag,
                CASE WHEN auto_recap_enabled THEN 1 ELSE 0 END AS auto_recap_enabled_flag,
                last_recap_at,
                updated_at
         FROM recap_configs
         WHERE enabled = TRUE
           AND auto_recap_enabled = TRUE
           AND (last_recap_at IS NULL OR last_recap_at <= $1)",
    )
    .bind(due_before)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(map_recap_config_row).collect())
}

pub async fn set_enabled(pool: &AnyPool, chat_id: i64, enabled: bool) -> Result<()> {
    sqlx::query("UPDATE recap_configs SET enabled = $1, updated_at = $2 WHERE chat_id = $3")
        .bind(enabled)
        .bind(now_ts())
        .bind(chat_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_auto_recap(pool: &AnyPool, chat_id: i64, enabled: bool) -> Result<()> {
    sqlx::query(
        "UPDATE recap_configs SET auto_recap_enabled = $1, updated_at = $2 WHERE chat_id = $3",
    )
    .bind(enabled)
    .bind(now_ts())
    .bind(chat_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn map_recap_config_row(row: sqlx::any::AnyRow) -> RecapConfig {
    RecapConfig {
        chat_id: row.get("chat_id"),
        enabled: row.get::<i64, _>("enabled_flag") != 0,
        auto_recap_enabled: row.get::<i64, _>("auto_recap_enabled_flag") != 0,
        last_recap_at: row.try_get("last_recap_at").ok(),
        updated_at: row.try_get("updated_at").ok(),
    }
}
