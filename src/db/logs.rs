use super::models::RecapLog;
use anyhow::Result;
use sqlx::AnyPool;

pub async fn insert_log(pool: &AnyPool, log: &RecapLog) -> Result<()> {
    sqlx::query(
        "INSERT INTO recap_logs (id, chat_id, prompt, recap_text, model, prompt_tokens, completion_tokens, feedback, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(&log.id)
    .bind(log.chat_id)
    .bind(&log.prompt)
    .bind(&log.recap_text)
    .bind(&log.model)
    .bind(log.prompt_tokens)
    .bind(log.completion_tokens)
    .bind(&log.feedback)
    .bind(log.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn add_feedback(pool: &AnyPool, log_id: String, feedback: String) -> Result<()> {
    sqlx::query("UPDATE recap_logs SET feedback = $1 WHERE id = $2")
        .bind(feedback)
        .bind(log_id)
        .execute(pool)
        .await?;
    Ok(())
}
