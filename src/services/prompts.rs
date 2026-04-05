// Prompt templates for OpenAI summarization, ported from Go version.

use serde::{Deserialize, Serialize};

// Default sarcastic condensed prompts (English fallback, can be overridden via environment variables or i18n).
pub const DEFAULT_SARCASTIC_SYSTEM_PROMPT: &str = "You are a concise chat history summarizer.\nSummarize the provided chat history into a single-sentence core summary, using 1-2 relevant emojis.\nBe concise and get straight to the point.\nRespond directly with the summary, no preamble or explanation.";

pub const DEFAULT_SARCASTIC_USER_PROMPT: &str = "Here is a chat history, please provide your summary:\n\nChat history:\"\"\"\n{{chat_history}}\n\"\"\"\n\nPlease provide the summary directly, no explanation.";

// Structured chat history summarization with JSON Schema output.
pub const CHAT_HISTORY_SUMMARIZATION_SYSTEM_PROMPT: &str = r#"You are an expert in summarizing refined outlines from documents and dialogues. Your task is to identify 1-20 distinct discussion topics from chat histories, focusing on key points and maintaining the conversation's essence.

Please format your response according to the following JSON Schema:
{"$schema":"http://json-schema.org/draft-07/schema#","title":"Chat Histories Summarization Schema","type":"array","items":{"type":"object","properties":{"topicName":{"type":"string","description":"The title, brief short title of the topic that talked, discussed in the chat history."},"sinceId":{"type":"number","description":"The id of the message from which the topic initially starts."},"participants":{"type":"array","description":"The list of the names of the participated users in the topic.","items":{"type":"string"}},"discussion":{"type":"array","description":"The list of the points that discussed during the topic.","items":{"type":"object","properties":{"point":{"type":"string","description":"The key point that talked, expressed, mentioned, or discussed during the topic."},"keyIds":{"type":"array","description":"The list of the ids of the messages that contain the key point.","items":{"type":"number"}}},"required":["point","keyIds"]},"minItems": 1,"maxItems": 5},"conclusion":{"type":"string","description":"The conclusion of the topic, optional."}},"required":["topicName","sinceId","participants","discussion"]}}

Example output:
[{"topicName":"Most Important Topic 1","sinceId":123456789,"participants":["John","Mary"],"discussion":[{"point":"Most relevant key point","keyIds":[123456789,987654321]}],"conclusion":"Optional brief conclusion"},{"topicName":"Most Important Topic 2","sinceId":987654321,"participants":["Bob","Alice"],"discussion":[{"point":"Most relevant key point","keyIds":[987654321]}],"conclusion":"Optional brief conclusion"}]"#;

pub const CHAT_HISTORY_SUMMARIZATION_USER_PROMPT: &str = r#"Please analyze the following chat history and provide a summary in {{language}}:

Chat histories:"""
{{chat_history}}
"""

Note: Topics may be discussed in parallel, so consider relevant keywords across the chat histories. Be concise and focus on the key essence of each topic."#;

/// Configurable prompts that can be overridden via environment variables.
#[derive(Debug, Clone)]
pub struct PromptConfig {
    pub sarcastic_system_prompt: String,
    pub sarcastic_user_prompt: String,
    pub summarization_language: String,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            sarcastic_system_prompt: DEFAULT_SARCASTIC_SYSTEM_PROMPT.to_string(),
            sarcastic_user_prompt: DEFAULT_SARCASTIC_USER_PROMPT.to_string(),
            summarization_language: "English".to_string(),
        }
    }
}

impl PromptConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(prompt) = std::env::var("SARCASTIC_CONDENSED_SYSTEM_PROMPT")
            && !prompt.is_empty()
        {
            config.sarcastic_system_prompt = prompt;
        }

        if let Ok(prompt) = std::env::var("SARCASTIC_CONDENSED_USER_PROMPT")
            && !prompt.is_empty()
        {
            config.sarcastic_user_prompt = prompt;
        }

        if let Ok(lang) = std::env::var("CHAT_HISTORIES_SUMMARIZATION_LANGUAGE")
            && !lang.is_empty()
        {
            config.summarization_language = lang;
        }

        config
    }

    /// Render the sarcastic user prompt with chat history substitution.
    pub fn render_sarcastic_user_prompt(&self, chat_history: &str) -> String {
        self.sarcastic_user_prompt
            .replace("{{chat_history}}", chat_history)
    }

}

// Structured output types for JSON summarization mode.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionPoint {
    pub point: String,
    #[serde(rename = "keyIds")]
    pub key_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSummary {
    #[serde(rename = "topicName")]
    pub topic_name: String,
    #[serde(rename = "sinceId")]
    pub since_id: i64,
    pub participants: Vec<String>,
    pub discussion: Vec<DiscussionPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conclusion: Option<String>,
}

pub type StructuredSummary = Vec<TopicSummary>;
