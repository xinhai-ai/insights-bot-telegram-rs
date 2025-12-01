use anyhow::{Context, Result};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
        CreateChatCompletionRequestArgs,
    },
};

use crate::{config::OpenAiConfig as OpenAiSettings, db::models::ChatHistory};

#[derive(Clone)]
pub struct OpenAiClient {
    client: Client<OpenAIConfig>,
    pub model: String,
    token_limit: Option<u32>,
}

impl OpenAiClient {
    pub fn new(cfg: &OpenAiSettings) -> Result<Self> {
        let mut builder = OpenAIConfig::new().with_api_key(&cfg.api_key);
        if let Some(base) = &cfg.api_base {
            builder = builder.with_api_base(base);
        }
        let client = Client::with_config(builder);
        Ok(Self {
            client,
            model: cfg.model.clone(),
            token_limit: cfg.token_limit,
        })
    }

    pub async fn recap(&self, history: &[ChatHistory]) -> Result<String> {
        let prompt_body = format_messages(history);

        let req = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(vec![
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                    content: "You are a concise Telegram chat recap assistant. Summarize key points, decisions, and action items. Respond in the main language used in the messages. Keep it under 10 bullet points."
                        .into(),
                    name: None,
                }),
                ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Text(prompt_body),
                    name: None,
                }),
            ])
            .max_tokens(self.token_limit.unwrap_or(800))
            .build()?;

        let resp = self
            .client
            .chat()
            .create(req)
            .await
            .context("openai chat completion failed")?;

        let content = resp
            .choices
            .get(0)
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| "Recap unavailable.".to_string());
        Ok(content)
    }

    // Placeholders for future media features.
    pub async fn analyze_image(
        &self,
        _image_bytes: &[u8],
        _user_prompt: Option<&str>,
    ) -> Result<String> {
        todo!("image analysis not implemented yet");
    }

    pub async fn transcribe_audio(&self, _audio_bytes: &[u8]) -> Result<String> {
        todo!("audio transcription not implemented yet");
    }
}

#[derive(Debug, Clone)]
pub struct RecapResult {
    pub text: String,
    pub model: String,
    pub created_at: i64,
}

fn format_messages(history: &[ChatHistory]) -> String {
    let mut lines = Vec::new();
    for h in history.iter().rev() {
        let sender = h
            .from_username
            .as_deref()
            .map(|u| format!("@{u}"))
            .unwrap_or_else(|| {
                h.from_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "unknown".into())
            });
        let body = h.text.clone().unwrap_or_default();
        lines.push(format!("[{}] {}", sender, body));
    }
    lines.join("\n")
}
