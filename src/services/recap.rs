use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    db::{
        Database, chat_history, logs,
        models::{RecapLog, RecapSubscription},
        recap_config,
    },
    services::openai::{OpenAiClient, RecapResult},
};

pub struct RecapService<'a> {
    pub db: &'a Database,
    pub openai: &'a OpenAiClient,
}

impl<'a> RecapService<'a> {
    pub fn new(db: &'a Database, openai: &'a OpenAiClient) -> Self {
        Self { db, openai }
    }

    pub async fn recap_chat(&self, chat_id: i64, limit: i64) -> Result<RecapResult> {
        let history = chat_history::recent_messages(&self.db.pool, chat_id, limit).await?;
        let text = self.openai.recap(&history).await?;
        let result = RecapResult {
            text: text.clone(),
            model: self.openai.model.clone(),
            created_at: Utc::now().timestamp(),
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

    pub async fn recap_forwarded(&self, user_id: i64, limit: i64) -> Result<RecapResult> {
        let history =
            chat_history::recent_forwarded_messages(&self.db.pool, user_id, limit).await?;
        let text = self.openai.recap(&history).await?;
        Ok(RecapResult {
            text,
            model: self.openai.model.clone(),
            created_at: Utc::now().timestamp(),
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
}
