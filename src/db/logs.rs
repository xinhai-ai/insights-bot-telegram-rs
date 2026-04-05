use super::models::RecapLog;
use anyhow::Result;
use sqlx::AnyPool;

pub async fn insert_log(pool: &AnyPool, log: &RecapLog) -> Result<()> {
    sqlx::query(
        "INSERT INTO recap_logs (id, chat_id, prompt, recap_text, model, prompt_tokens, completion_tokens, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(&log.id)
    .bind(log.chat_id)
    .bind(&log.prompt)
    .bind(&log.recap_text)
    .bind(&log.model)
    .bind(log.prompt_tokens)
    .bind(log.completion_tokens)
    .bind(log.created_at)
    .execute(pool)
    .await?;
    Ok(())
}
