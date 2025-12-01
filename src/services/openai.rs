use anyhow::Result;
use async_openai::{Client, config::OpenAIConfig};

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
        let _ = history;
        Ok("Recap not implemented yet.".to_string())
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
