use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    config::Locale,
    db::{
        Database, chat_history, logs,
        models::{ChatHistory, RecapLog, RecapSubscription},
        recap_config,
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
            feedback: None,
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

    #[allow(dead_code)]
    pub async fn recap_forwarded(
        &self,
        user_id: i64,
        limit: i64,
        i18n: &I18n,
    ) -> Result<RecapResult> {
        let forwarded = chat_history::list_forwarded(&self.db.pool, user_id, limit).await?;
        if forwarded.is_empty() {
            anyhow::bail!("no forwarded messages");
        }
        let history: Vec<ChatHistory> = forwarded
            .into_iter()
            .map(|f| ChatHistory {
                id: f.id,
                chat_id: f.from_chat_id.unwrap_or(0),
                message_id: f.message_id.unwrap_or(0),
                from_id: Some(f.user_id),
                from_full_name: String::new(),
                from_username: String::new(),
                kind: f.kind,
                text: f.text.unwrap_or_default(),
                media_url: String::new(),
                created_at: f.created_at,
            })
            .collect();
        let text = self.openai.recap(&history, i18n).await?;
        chat_history::clear_forwarded(&self.db.pool, user_id).await?;
        Ok(RecapResult {
            text,
            model: self.openai.model.clone(),
            created_at: Utc::now().timestamp(),
            sarcastic_summary: None,
        })
    }

    pub async fn subscribe(&self, chat_id: i64, user_id: i64) -> Result<()> {
        let sub = RecapSubscription {
            id: Uuid::new_v4().to_string(),
            chat_id,
            user_id,
            created_at: Some(Utc::now().timestamp()),
        };
        recap_config::add_subscription(&self.db.pool, &sub).await
    }

    pub async fn unsubscribe(&self, chat_id: i64, user_id: i64) -> Result<()> {
        recap_config::remove_subscription(&self.db.pool, chat_id, user_id).await
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
                feedback: None,
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
                feedback: None,
                created_at: Some(output.created_at),
            };
            logs::insert_log(&self.db.pool, &log).await?;
        }

        Ok(output)
    }
}
