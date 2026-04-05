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

use crate::{
    config::{Locale, OpenAiConfig as OpenAiSettings},
    db::models::ChatHistory,
    i18n::I18n,
    services::prompts::{
        CHAT_HISTORY_SUMMARIZATION_SYSTEM_PROMPT, CHAT_HISTORY_SUMMARIZATION_USER_PROMPT,
        PromptConfig, StructuredSummary, TopicSummary,
    },
};

#[derive(Clone)]
pub struct OpenAiClient {
    client: Client<OpenAIConfig>,
    pub model: String,
    pub sarcastic_model: Option<String>,
    token_limit: Option<u32>,
    pub prompt_config: PromptConfig,
}

impl OpenAiClient {
    pub fn new(cfg: &OpenAiSettings) -> Result<Self> {
        let mut builder = OpenAIConfig::new().with_api_key(&cfg.api_key);
        if let Some(base) = &cfg.api_base {
            builder = builder.with_api_base(base);
        }
        let client = Client::with_config(builder);
        let prompt_config = PromptConfig::from_env();
        let sarcastic_model = std::env::var("SARCASTIC_CONDENSED_MODEL_NAME").ok();

        Ok(Self {
            client,
            model: cfg.model.clone(),
            sarcastic_model,
            token_limit: cfg.token_limit,
            prompt_config,
        })
    }

    /// Sarcastic condensed single-sentence summary with emoji.
    pub async fn sarcastic_condense(&self, content: &str) -> Result<String> {
        let model = self.sarcastic_model.as_ref().unwrap_or(&self.model).clone();

        let user_prompt = self.prompt_config.render_sarcastic_user_prompt(content);

        let req = CreateChatCompletionRequestArgs::default()
            .model(&model)
            .messages(vec![
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                    content: self.prompt_config.sarcastic_system_prompt.clone().into(),
                    name: None,
                }),
                ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Text(user_prompt),
                    name: None,
                }),
            ])
            .max_tokens(200u32) // Short response expected.
            .build()?;

        let resp = self
            .client
            .chat()
            .create(req)
            .await
            .context("sarcastic condense failed")?;

        let text = resp
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| {
                tracing::warn!("sarcastic_condense: no content in API response");
                "Summary unavailable.".to_string()
            });

        tracing::debug!("sarcastic_condense raw response: {:?}", text);
        Ok(text.trim().to_string())
    }

    /// Structured JSON summarization with locale-aware output language.
    pub async fn recap_structured_locale(
        &self,
        content: &str,
        locale: &Locale,
    ) -> Result<StructuredSummary> {
        let language = match locale {
            Locale::ZhHans => "Simplified Chinese",
            Locale::ZhHant => "Traditional Chinese",
            Locale::En => "English",
        };

        let user_prompt = CHAT_HISTORY_SUMMARIZATION_USER_PROMPT
            .replace("{{language}}", language)
            .replace("{{chat_history}}", content);

        let req = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(vec![
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                    content: CHAT_HISTORY_SUMMARIZATION_SYSTEM_PROMPT.into(),
                    name: None,
                }),
                ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Text(user_prompt),
                    name: None,
                }),
            ])
            .max_tokens(self.token_limit.unwrap_or(8000))
            .build()?;

        let resp = self
            .client
            .chat()
            .create(req)
            .await
            .context("structured summarization failed")?;

        let raw_text = resp
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| "[]".to_string());

        tracing::debug!("recap_structured_locale raw response: {}", raw_text);

        // Try to extract JSON from response (may be wrapped in markdown code block)
        let json_text = extract_json_from_response(&raw_text);

        // Try to parse JSON, fallback to empty array on failure.
        let summary: StructuredSummary = serde_json::from_str(&json_text).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse structured summary JSON: {e}");
            Vec::new()
        });

        Ok(summary)
    }

    /// Generate both condensed and segmented summaries for chat history.
    pub async fn generate_dual_recap(
        &self,
        history: &[ChatHistory],
        locale: &Locale,
        chat_id: i64,
        i18n: &I18n,
    ) -> Result<RecapOutput> {
        let formatted = format_messages(history);
        if formatted.is_empty() {
            anyhow::bail!("no messages to summarize");
        }

        // Generate both summaries concurrently
        let (condensed_result, segmented_result) = tokio::join!(
            self.sarcastic_condense(&formatted),
            self.recap_structured_locale(&formatted, locale)
        );

        let condensed_summary = match condensed_result {
            Ok(text) => {
                if text.trim().is_empty() {
                    tracing::warn!("sarcastic_condense returned empty text");
                    "Summary generation failed".to_string()
                } else {
                    text
                }
            }
            Err(e) => {
                tracing::warn!("sarcastic_condense failed: {e:?}");
                "Summary generation failed".to_string()
            }
        };

        // Format structured topics to markdown (for Telegraph) and HTML (for Telegram inline)
        let (segmented_summary, segmented_summary_html) = match segmented_result {
            Ok(topics) => (
                format_topics_to_markdown(&topics, locale, chat_id, i18n),
                format_topics_to_telegram_html(&topics, locale, chat_id, i18n),
            ),
            Err(e) => {
                tracing::warn!("recap_structured_locale failed: {e:?}");
                let fallback = "Segmented summary generation failed".to_string();
                (fallback.clone(), fallback)
            }
        };

        let condensed_model = self
            .sarcastic_model
            .clone()
            .unwrap_or_else(|| self.model.clone());

        Ok(RecapOutput {
            condensed_summary,
            segmented_summary,
            segmented_summary_html,
            condensed_model,
            segmented_model: self.model.clone(),
            created_at: chrono::Utc::now().timestamp(),
        })
    }
}

/// Full recap output with condensed and segmented summaries.
#[derive(Debug, Clone)]
pub struct RecapOutput {
    /// Condensed single-sentence summary with emoji.
    pub condensed_summary: String,
    /// Full segmented summary in Markdown+HTML (for Telegraph nodes).
    pub segmented_summary: String,
    /// Segmented summary in pure Telegram HTML (for inline messages).
    pub segmented_summary_html: String,
    /// Model used for condensed summary.
    pub condensed_model: String,
    /// Model used for segmented summary.
    pub segmented_model: String,
    pub created_at: i64,
}

/// Format user name for display: prefer full_name, fallback to username if full_name is too long.
fn format_user_name(full_name: &str, username: &str) -> String {
    // If full_name is >= 10 chars and username exists, use username
    if full_name.chars().count() >= 10 && !username.is_empty() {
        return username.to_string();
    }
    // Remove # characters from full_name
    if !full_name.is_empty() {
        return full_name.replace('#', "");
    }
    if !username.is_empty() {
        return username.to_string();
    }
    "unknown".to_string()
}

/// Format chat history messages for LLM input.
/// Uses format: `msgId:{id}: {name} sent: {text}`
fn format_messages(history: &[ChatHistory]) -> String {
    let mut lines = Vec::new();
    for h in history.iter() {
        // Skip empty text messages
        if h.text.is_empty() {
            continue;
        }
        let sender = format_user_name(&h.from_full_name, &h.from_username);
        // Format: msgId:{id}: {name} sent: {text}
        lines.push(format!(
            "msgId:{}: {} sent: {}",
            h.message_id, sender, h.text
        ));
    }
    lines.join("\n")
}

/// Extract JSON from response that may be wrapped in markdown code block.
fn extract_json_from_response(text: &str) -> String {
    let trimmed = text.trim();

    // Try to extract from markdown code block
    if trimmed.starts_with("```") {
        // Find the end of the first line (language specifier)
        if let Some(first_newline) = trimmed.find('\n') {
            let after_lang = &trimmed[first_newline + 1..];
            // Find closing ```
            if let Some(end_pos) = after_lang.rfind("```") {
                return after_lang[..end_pos].trim().to_string();
            }
        }
    }

    // Return as-is if not wrapped
    trimmed.to_string()
}

/// Escape HTML special characters for Telegram HTML parse mode.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Format structured topics into Telegram-compatible HTML (for inline messages).
/// Uses `<b>` for headers, preserves `<a>` links, and escapes AI-generated text.
pub fn format_topics_to_telegram_html(
    topics: &[TopicSummary],
    locale: &Locale,
    chat_id: i64,
    i18n: &I18n,
) -> String {
    if topics.is_empty() {
        return "No discussion topics identified.".to_string();
    }

    let participants_label = i18n.t(*locale, "labels.participants", &[]);
    let discussion_label = i18n.t(*locale, "labels.discussion", &[]);
    let conclusion_label = i18n.t(*locale, "labels.conclusion", &[]);
    let colon = i18n.t(*locale, "labels.colon", &[]);
    let comma = i18n.t(*locale, "labels.comma", &[]);

    let chat_cid = if chat_id < 0 {
        (chat_id.abs() - 1_000_000_000_000).to_string()
    } else {
        chat_id.to_string()
    };

    let mut output = Vec::new();

    for topic in topics {
        // Topic title: <b> with optional <a> link
        if topic.since_id > 0 {
            output.push(format!(
                "<b><a href=\"https://t.me/c/{}/{}\">{}</a></b>",
                chat_cid, topic.since_id, escape_html(&topic.topic_name)
            ));
        } else {
            output.push(format!("<b>{}</b>", escape_html(&topic.topic_name)));
        }

        // Participants
        let participants_str = topic
            .participants
            .iter()
            .map(|p| escape_html(p))
            .collect::<Vec<_>>()
            .join(&comma);
        output.push(format!("{}{}{}", participants_label, colon, participants_str));

        // Discussion
        output.push(format!("{}{}", discussion_label, colon));

        for point in &topic.discussion {
            let links: Vec<String> = point
                .key_ids
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    format!(
                        "<a href=\"https://t.me/c/{}/{}\">[{}]</a>",
                        chat_cid, id, i + 1
                    )
                })
                .collect();

            let links_str = if links.is_empty() {
                String::new()
            } else {
                format!(" {}", links.join(" "))
            };

            output.push(format!(" • {}{}", escape_html(&point.point), links_str));
        }

        // Conclusion (optional)
        if let Some(conclusion) = &topic.conclusion
            && !conclusion.is_empty()
        {
            output.push(format!(
                "{}{}{}",
                conclusion_label,
                colon,
                escape_html(conclusion)
            ));
        }

        output.push(String::new());
    }

    output.join("\n")
}

/// Format structured topics into Markdown text with locale-aware labels.
pub fn format_topics_to_markdown(
    topics: &[TopicSummary],
    locale: &Locale,
    chat_id: i64,
    i18n: &I18n,
) -> String {
    if topics.is_empty() {
        return "No discussion topics identified.".to_string();
    }

    // Locale-specific labels and punctuation from i18n
    let participants_label = i18n.t(*locale, "labels.participants", &[]);
    let discussion_label = i18n.t(*locale, "labels.discussion", &[]);
    let conclusion_label = i18n.t(*locale, "labels.conclusion", &[]);
    let colon = i18n.t(*locale, "labels.colon", &[]);
    let comma = i18n.t(*locale, "labels.comma", &[]);

    // Convert chat_id to t.me/c/ format (remove -100 prefix for supergroups)
    let chat_cid = if chat_id < 0 {
        (chat_id.abs() - 1_000_000_000_000).to_string()
    } else {
        chat_id.to_string()
    };

    let mut output = Vec::new();

    for topic in topics {
        // Topic title with optional link to since_id
        if topic.since_id > 0 {
            output.push(format!(
                "## <a href=\"https://t.me/c/{}/{}\">{}</a>",
                chat_cid, topic.since_id, topic.topic_name
            ));
        } else {
            output.push(format!("## {}", topic.topic_name));
        }

        // Participants
        let participants_str = topic.participants.join(&comma);
        output.push(format!(
            "{}{}{}",
            participants_label, colon, participants_str
        ));

        // Discussion
        output.push(format!("{}{}", discussion_label, colon));

        for point in &topic.discussion {
            // Format key_ids as links
            let links: Vec<String> = point
                .key_ids
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    format!(
                        "<a href=\"https://t.me/c/{}/{}\">[{}]</a>",
                        chat_cid,
                        id,
                        i + 1
                    )
                })
                .collect();

            let links_str = if links.is_empty() {
                String::new()
            } else {
                format!(" {}", links.join(" "))
            };

            output.push(format!(" - {}{}", point.point, links_str));
        }

        // Conclusion (optional)
        if let Some(conclusion) = &topic.conclusion
            && !conclusion.is_empty()
        {
            output.push(format!("{}{}{}", conclusion_label, colon, conclusion));
        }

        output.push(String::new()); // Empty line between topics
    }

    output.join("\n")
}
