use anyhow::Result;
use sqlx::AnyPool;

use super::models::{RecapConfig, RecapSubscription};

pub async fn upsert_recap_config(pool: &AnyPool, cfg: &RecapConfig) -> Result<()> {
    sqlx::query(
        "INSERT INTO recap_configs (chat_id, enabled, mode, auto_recap_enabled, auto_recap_rates_per_day, last_recap_at, pinned_message_id, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
         ON CONFLICT (chat_id) DO UPDATE SET
           enabled = EXCLUDED.enabled,
           mode = EXCLUDED.mode,
           auto_recap_enabled = EXCLUDED.auto_recap_enabled,
           auto_recap_rates_per_day = EXCLUDED.auto_recap_rates_per_day,
           last_recap_at = EXCLUDED.last_recap_at,
           pinned_message_id = EXCLUDED.pinned_message_id,
           updated_at = EXCLUDED.updated_at",
    )
    .bind(cfg.chat_id)
    .bind(cfg.enabled)
    .bind(&cfg.mode)
    .bind(cfg.auto_recap_enabled)
    .bind(cfg.auto_recap_rates_per_day)
    .bind(cfg.last_recap_at)
    .bind(cfg.pinned_message_id)
    .bind(cfg.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_recap_config(pool: &AnyPool, chat_id: i64) -> Result<Option<RecapConfig>> {
    let rec =
        sqlx::query_as::<_, RecapConfig>("SELECT * FROM recap_configs WHERE chat_id = $1 LIMIT 1")
            .bind(chat_id)
            .fetch_optional(pool)
            .await?;
    Ok(rec)
}

pub async fn get_or_create_recap_config(pool: &AnyPool, chat_id: i64) -> Result<RecapConfig> {
    if let Some(cfg) = get_recap_config(pool, chat_id).await? {
        return Ok(cfg);
    }
    let default = RecapConfig {
        chat_id,
        enabled: true,
        mode: None,
        auto_recap_enabled: false,
        auto_recap_rates_per_day: Some(1),
        last_recap_at: None,
        pinned_message_id: None,
        updated_at: None,
    };
    upsert_recap_config(pool, &default).await?;
    Ok(default)
}

pub async fn list_due_for_auto_recap(pool: &AnyPool, now: i64) -> Result<Vec<RecapConfig>> {
    let rows = sqlx::query_as::<_, RecapConfig>(
        "SELECT * FROM recap_configs
         WHERE auto_recap_enabled = TRUE
           AND (last_recap_at IS NULL OR last_recap_at <= $1)",
    )
    .bind(now)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn add_subscription(pool: &AnyPool, sub: &RecapSubscription) -> Result<()> {
    sqlx::query(
        "INSERT INTO recap_subscriptions (id, chat_id, user_id, created_at)
         VALUES ($1,$2,$3,$4)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(&sub.id)
    .bind(sub.chat_id)
    .bind(sub.user_id)
    .bind(sub.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_subscription(pool: &AnyPool, chat_id: i64, user_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM recap_subscriptions WHERE chat_id = $1 AND user_id = $2")
        .bind(chat_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_subscribers(pool: &AnyPool, chat_id: i64) -> Result<Vec<RecapSubscription>> {
    let rows = sqlx::query_as::<_, RecapSubscription>(
        "SELECT * FROM recap_subscriptions WHERE chat_id = $1",
    )
    .bind(chat_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
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

pub async fn set_rates_per_day(pool: &AnyPool, chat_id: i64, rates: i32) -> Result<()> {
    sqlx::query(
        "UPDATE recap_configs SET auto_recap_rates_per_day = $1, updated_at = $2 WHERE chat_id = $3",
    )
    .bind(rates)
    .bind(now_ts())
    .bind(chat_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}
