use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    config::Locale,
    db::{
        Database, chat_history, logs,
        models::{ChatHistory, RecapLog},
    },
    i18n::I18n,
    services::openai::{OpenAiClient, RecapOutput, RecapResult},
};

pub struct RecapService<'a> {
    pub db: &'a Database,
    pub openai: &'a OpenAiClient,
}

impl<'a> RecapService<'a> {
    pub fn new(db: &'a Database, openai: &'a OpenAiClient) -> Self {
        Self { db, openai }
    }

    #[allow(dead_code)]
    pub async fn recap_chat(&self, chat_id: i64, limit: i64, i18n: &I18n) -> Result<RecapResult> {
        let history = chat_history::recent_messages(&self.db.pool, chat_id, limit).await?;
        let text = self.openai.recap(&history, i18n).await?;
        let result = RecapResult {
            text: text.clone(),
            model: self.openai.model.clone(),
            created_at: Utc::now().timestamp(),
            sarcastic_summary: None,
        };

        let log = RecapLog {
            id: Uuid::new_v4().to_string(),
            chat_id,
            prompt: None,
            recap_text: Some(text),
            model: Some(result.model.clone()),
            prompt_tokens: None,
            completion_tokens: None,
            created_at: Some(result.created_at),
        };
        logs::insert_log(&self.db.pool, &log).await?;
        Ok(result)
    }

    /// Generate dual recap (condensed + segmented) for a chat_id by fetching recent history.
    pub async fn recap_chat_dual(
        &self,
        chat_id: i64,
        limit: i64,
        locale: &Locale,
        i18n: &I18n,
    ) -> Result<RecapOutput> {
        let history = chat_history::recent_messages(&self.db.pool, chat_id, limit).await?;
        self.generate_dual_recap(&history, locale, chat_id, i18n)
            .await
    }

    /// Generate recap from provided messages with locale-aware prompts.
    #[allow(dead_code)]
    pub async fn recap_messages(
        &self,
        messages: &[ChatHistory],
        locale: &Locale,
        i18n: &I18n,
    ) -> Result<RecapResult> {
        let text = self
            .openai
            .recap_with_locale(messages, locale, i18n)
            .await?;
        let result = RecapResult {
            text: text.clone(),
            model: self.openai.model.clone(),
            created_at: Utc::now().timestamp(),
            sarcastic_summary: None,
        };

        // Log the recap (use first message's chat_id if available).
        if let Some(first) = messages.first() {
            let log = RecapLog {
                id: Uuid::new_v4().to_string(),
                chat_id: first.chat_id,
                prompt: None,
                recap_text: Some(text),
                model: Some(result.model.clone()),
                prompt_tokens: None,
                completion_tokens: None,
                created_at: Some(result.created_at),
            };
            logs::insert_log(&self.db.pool, &log).await?;
        }

        Ok(result)
    }

    /// Generate dual recap (condensed + segmented) from provided messages.
    /// Returns RecapOutput with both summaries for Telegraph publishing.
    pub async fn generate_dual_recap(
        &self,
        messages: &[ChatHistory],
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
                model: Some(output.segmented_model.clone()),
                prompt_tokens: None,
                completion_tokens: None,
                created_at: Some(output.created_at),
            };
            logs::insert_log(&self.db.pool, &log).await?;
        }

        Ok(output)
    }
}
