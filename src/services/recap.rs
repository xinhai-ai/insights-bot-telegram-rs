use anyhow::Result;
use uuid::Uuid;

use crate::{
    config::Locale,
    db::{
        Database, logs,
        models::RecapLog,
    },
    i18n::I18n,
    services::openai::{OpenAiClient, RecapOutput},
};

pub struct RecapService<'a> {
    pub db: &'a Database,
    pub openai: &'a OpenAiClient,
}

impl<'a> RecapService<'a> {
    pub fn new(db: &'a Database, openai: &'a OpenAiClient) -> Self {
        Self { db, openai }
    }

    /// Generate dual recap (condensed + segmented) from provided messages.
    /// Returns RecapOutput with both summaries for Telegraph publishing.
    pub async fn generate_dual_recap(
        &self,
        messages: &[crate::db::models::ChatHistory],
        locale: &Locale,
        chat_id: i64,
        i18n: &I18n,
    ) -> Result<RecapOutput> {
        let output = self
            .openai
            .generate_dual_recap(messages, locale, chat_id, i18n)
            .await?;

        // Log the recap
        if let Some(first) = messages.first() {
            let log = RecapLog {
                id: Uuid::new_v4().to_string(),
                chat_id: first.chat_id,
                prompt: None,
                recap_text: Some(output.segmented_summary.clone()),
                model: Some(output.trace.segmented_model.clone()),
                prompt_tokens: None,
                completion_tokens: None,
                created_at: Some(output.created_at),
            };
            logs::insert_log(&self.db.pool, &log).await?;
        }

        Ok(output)
    }
}
