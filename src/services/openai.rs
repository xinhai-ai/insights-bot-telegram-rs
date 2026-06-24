use anyhow::{Context, Result};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
        CreateChatCompletionRequest, CreateChatCompletionRequestArgs,
    },
};
use futures_util::StreamExt;

use crate::{
    config::{Locale, OpenAiConfig as OpenAiSettings},
    db::models::ChatHistory,
    i18n::I18n,
    services::prompts::{
        CHAT_HISTORY_SUMMARIZATION_SYSTEM_PROMPT, CHAT_HISTORY_SUMMARIZATION_USER_PROMPT,
        CHECK_CONDENSED_OUTPUT_SYSTEM_PROMPT, CHECK_CONDENSED_OUTPUT_USER_PROMPT,
        CHECK_SUMMARY_JSON_SYSTEM_PROMPT, CHECK_SUMMARY_JSON_USER_PROMPT, PromptConfig,
        StructuredSummary, TopicSummary,
    },
};

const CHAT_MESSAGES_PER_USER_PROMPT: usize = 20;

#[derive(Clone)]
pub struct OpenAiClient {
    client: Client<OpenAIConfig>,
    pub model: String,
    pub sarcastic_model: Option<String>,
    pub check_model: Option<String>,
    pub check_model_backup: Option<String>,
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
        let check_model = std::env::var("CHECK_MODEL").ok().filter(|s| !s.is_empty());
        let check_model_backup = std::env::var("CHECK_MODEL_backup")
            .ok()
            .filter(|s| !s.is_empty());

        Ok(Self {
            client,
            model: cfg.model.clone(),
            sarcastic_model,
            check_model,
            check_model_backup,
            token_limit: cfg.token_limit,
            prompt_config,
        })
    }

    async fn stream_chat_completion(
        &self,
        req: CreateChatCompletionRequest,
        context: &'static str,
    ) -> Result<String> {
        let mut stream = self
            .client
            .chat()
            .create_stream(req)
            .await
            .context(context)?;
        let mut text = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context(context)?;
            for choice in chunk.choices {
                if let Some(content) = choice.delta.content {
                    text.push_str(&content);
                }
            }
        }

        Ok(text)
    }

    /// Sarcastic condensed single-sentence summary with emoji.
    pub async fn sarcastic_condense(
        &self,
        history: &[ChatHistory],
        locale: &Locale,
        i18n: &I18n,
    ) -> Result<String> {
        let model = self.sarcastic_model.as_ref().unwrap_or(&self.model).clone();
        let history_chunks = format_message_chunks(history, CHAT_MESSAGES_PER_USER_PROMPT);
        if history_chunks.is_empty() {
            anyhow::bail!("no messages to summarize");
        }
        let history_notice = chunked_history_notice(locale);

        let system_prompt = i18n
            .t(*locale, "prompts.sarcastic_system", &[])
            .replace("{{chat_history}}", history_notice)
            .replace("{chat_history}", history_notice);
        let user_prompt = i18n
            .t(*locale, "prompts.sarcastic_user", &[])
            .replace("{{chat_history}}", history_notice)
            .replace("{chat_history}", history_notice);

        let system_prompt = if system_prompt == "prompts.sarcastic_system" {
            self.prompt_config
                .render_sarcastic_system_prompt(history_notice)
        } else {
            system_prompt
        };
        let user_prompt = if user_prompt == "prompts.sarcastic_user" {
            self.prompt_config
                .render_sarcastic_user_prompt(history_notice)
        } else {
            user_prompt
        };

        let mut messages = vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: system_prompt.into(),
                name: None,
            }),
            user_message(user_prompt),
        ];
        append_chat_history_chunks(&mut messages, &history_chunks, Some(locale));

        let req = CreateChatCompletionRequestArgs::default()
            .model(&model)
            .messages(messages)
            .build()?;

        let text = self
            .stream_chat_completion(req, "sarcastic condense failed")
            .await?;

        if text.trim().is_empty() {
            tracing::warn!("sarcastic_condense: no content in API response");
            return Ok("Summary unavailable.".to_string());
        }

        tracing::debug!("sarcastic_condense raw response: {:?}", text);
        Ok(text.trim().to_string())
    }

    /// Structured JSON summarization with locale-aware output language.
    /// Returns (parsed topics, raw JSON text) — raw text is kept for check model repair.
    pub async fn recap_structured_locale(
        &self,
        history: &[ChatHistory],
        locale: &Locale,
    ) -> Result<(StructuredSummary, String)> {
        let history_chunks = format_message_chunks(history, CHAT_MESSAGES_PER_USER_PROMPT);
        if history_chunks.is_empty() {
            anyhow::bail!("no messages to summarize");
        }
        let history_notice = chunked_history_notice(locale);

        let language = match locale {
            Locale::ZhHans => "Simplified Chinese",
            Locale::ZhHant => "Traditional Chinese",
            Locale::En => "English",
        };

        let user_prompt = CHAT_HISTORY_SUMMARIZATION_USER_PROMPT
            .replace("{{language}}", language)
            .replace("{{chat_history}}", history_notice)
            .replace("{chat_history}", history_notice);

        let mut messages = vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: CHAT_HISTORY_SUMMARIZATION_SYSTEM_PROMPT.into(),
                name: None,
            }),
            user_message(user_prompt),
        ];
        append_chat_history_chunks(&mut messages, &history_chunks, None);

        let req = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .max_tokens(self.token_limit.unwrap_or(8000))
            .build()?;

        let raw_text = self
            .stream_chat_completion(req, "structured summarization failed")
            .await?;
        let raw_text = if raw_text.trim().is_empty() {
            "[]".to_string()
        } else {
            raw_text
        };

        tracing::debug!("recap_structured_locale raw response: {}", raw_text);

        // Try to extract JSON from response (may be wrapped in markdown code block)
        let json_text = extract_json_from_response(&raw_text);

        // Try to parse JSON, return raw text alongside for potential check model repair.
        let summary: StructuredSummary = serde_json::from_str(&json_text).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse structured summary JSON: {e}");
            Vec::new()
        });

        Ok((summary, json_text))
    }

    /// Send a single chat completion request to the specified model for repair.
    async fn call_check_model(
        &self,
        model_name: &str,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        let req = CreateChatCompletionRequestArgs::default()
            .model(model_name)
            .messages(vec![
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                    content: system_prompt.into(),
                    name: None,
                }),
                user_message(user_prompt.to_string()),
            ])
            .max_tokens(2000u32)
            .build()?;

        let text = self
            .stream_chat_completion(req, "check model call failed")
            .await?;

        let text = text.trim();
        if text.is_empty() {
            anyhow::bail!("check model returned no content");
        }
        Ok(text.to_string())
    }

    /// Attempt to repair malformed segmented JSON using check model (+ backup).
    async fn repair_segmented_json(
        &self,
        raw_json: &str,
        trace: &mut CheckModelTrace,
    ) -> Option<StructuredSummary> {
        let check_model = self.check_model.as_ref()?;
        trace.model = check_model.clone();
        if let Some(backup) = &self.check_model_backup {
            trace.backup_model = backup.clone();
        }
        trace.attempted = true;

        let user_prompt = CHECK_SUMMARY_JSON_USER_PROMPT.replace("{{raw_json}}", raw_json);

        // Try primary check model
        if let Ok(repaired) = self
            .call_check_model(check_model, CHECK_SUMMARY_JSON_SYSTEM_PROMPT, &user_prompt)
            .await
        {
            let cleaned = extract_json_from_response(&repaired);
            if let Ok(summary) = serde_json::from_str::<StructuredSummary>(&cleaned) {
                tracing::info!("check model repaired segmented JSON (primary)");
                trace.succeeded = true;
                return Some(summary);
            }
        }

        // Try backup if available
        if let Some(backup) = &self.check_model_backup {
            trace.backup_used = true;
            if let Ok(repaired) = self
                .call_check_model(backup, CHECK_SUMMARY_JSON_SYSTEM_PROMPT, &user_prompt)
                .await
            {
                let cleaned = extract_json_from_response(&repaired);
                if let Ok(summary) = serde_json::from_str::<StructuredSummary>(&cleaned) {
                    tracing::info!("check model repaired segmented JSON (backup: {backup})");
                    trace.succeeded = true;
                    trace.backup_succeeded = true;
                    return Some(summary);
                }
            }
        }

        tracing::warn!("check model failed to repair segmented JSON");
        trace.failed = true;
        None
    }

    /// Attempt to repair malformed condensed output using check model (+ backup).
    async fn repair_condensed_output(
        &self,
        raw_output: &str,
        trace: &mut CheckModelTrace,
    ) -> Option<String> {
        let check_model = self.check_model.as_ref()?;
        trace.model = check_model.clone();
        if let Some(backup) = &self.check_model_backup {
            trace.backup_model = backup.clone();
        }
        trace.attempted = true;

        let user_prompt = CHECK_CONDENSED_OUTPUT_USER_PROMPT.replace("{{raw_output}}", raw_output);

        // Try primary check model
        if let Ok(repaired) = self
            .call_check_model(
                check_model,
                CHECK_CONDENSED_OUTPUT_SYSTEM_PROMPT,
                &user_prompt,
            )
            .await
            && !needs_condensed_repair(&repaired)
        {
            tracing::info!("check model repaired condensed output (primary)");
            trace.succeeded = true;
            return Some(repaired);
        }

        // Try backup if available
        if let Some(backup) = &self.check_model_backup {
            trace.backup_used = true;
            if let Ok(repaired) = self
                .call_check_model(backup, CHECK_CONDENSED_OUTPUT_SYSTEM_PROMPT, &user_prompt)
                .await
                && !needs_condensed_repair(&repaired)
            {
                tracing::info!("check model repaired condensed output (backup: {backup})");
                trace.succeeded = true;
                trace.backup_succeeded = true;
                return Some(repaired);
            }
        }

        tracing::warn!("check model failed to repair condensed output");
        trace.failed = true;
        None
    }

    /// Generate both condensed and segmented summaries for chat history.
    pub async fn generate_dual_recap(
        &self,
        history: &[ChatHistory],
        locale: &Locale,
        chat_id: i64,
        i18n: &I18n,
    ) -> Result<RecapOutput> {
        if format_message_chunks(history, CHAT_MESSAGES_PER_USER_PROMPT).is_empty() {
            anyhow::bail!("no messages to summarize");
        }

        let condensed_model_name = self
            .sarcastic_model
            .clone()
            .unwrap_or_else(|| self.model.clone());

        // Initialize trace
        let mut trace = RecapTrace {
            condensed_model: condensed_model_name,
            segmented_model: self.model.clone(),
            check: CheckModelTrace {
                model: self.check_model.clone().unwrap_or_default(),
                backup_model: self.check_model_backup.clone().unwrap_or_default(),
                ..Default::default()
            },
        };

        // Generate both summaries concurrently
        let (condensed_result, segmented_result) = tokio::join!(
            self.sarcastic_condense(history, locale, i18n),
            self.recap_structured_locale(history, locale)
        );

        // Process condensed result, optionally repair with check model
        let mut condensed_summary = match condensed_result {
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

        // Check model repair for condensed output
        if needs_condensed_repair(&condensed_summary) && self.check_model.is_some() {
            tracing::info!("condensed output needs repair, invoking check model");
            if let Some(repaired) = self
                .repair_condensed_output(&condensed_summary, &mut trace.check)
                .await
            {
                condensed_summary = repaired;
            }
        }

        // Process segmented result, optionally repair with check model
        let (segmented_summary, segmented_summary_html) = match segmented_result {
            Ok((topics, raw_json)) => {
                if topics.is_empty() && !raw_json.is_empty() && self.check_model.is_some() {
                    // JSON parsing failed, try check model repair
                    tracing::info!("segmented JSON parsing failed, invoking check model");
                    if let Some(repaired_topics) = self
                        .repair_segmented_json(&raw_json, &mut trace.check)
                        .await
                    {
                        (
                            format_topics_to_markdown(&repaired_topics, locale, chat_id, i18n),
                            format_topics_to_telegram_html(&repaired_topics, locale, chat_id, i18n),
                        )
                    } else {
                        let fallback = "No discussion topics identified.".to_string();
                        (fallback.clone(), fallback)
                    }
                } else if topics.is_empty() {
                    let fallback = "No discussion topics identified.".to_string();
                    (fallback.clone(), fallback)
                } else {
                    (
                        format_topics_to_markdown(&topics, locale, chat_id, i18n),
                        format_topics_to_telegram_html(&topics, locale, chat_id, i18n),
                    )
                }
            }
            Err(e) => {
                tracing::warn!("recap_structured_locale failed: {e:?}");
                let fallback = "Segmented summary generation failed".to_string();
                (fallback.clone(), fallback)
            }
        };

        Ok(RecapOutput {
            condensed_summary,
            segmented_summary,
            segmented_summary_html,
            trace,
            created_at: chrono::Utc::now().timestamp(),
        })
    }
}

/// Check if condensed output is malformed and needs check model repair.
/// Ported from Go `invalidCondensedOutputReason()`.
fn needs_condensed_repair(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.contains("```") {
        return true;
    }
    // JSON-like detection
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return true;
    }
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        return true;
    }
    false
}

/// Tracks whether the check model was invoked and its outcome.
#[derive(Debug, Clone, Default)]
pub struct CheckModelTrace {
    pub model: String,
    pub backup_model: String,
    pub attempted: bool,
    pub succeeded: bool,
    pub failed: bool,
    pub backup_used: bool,
    pub backup_succeeded: bool,
}

/// Full execution trace for a recap generation, paralleling Go's structure.
#[derive(Debug, Clone, Default)]
pub struct RecapTrace {
    pub condensed_model: String,
    pub segmented_model: String,
    pub check: CheckModelTrace,
}

impl RecapTrace {
    /// Build the three-line model status footer, joined by newline.
    pub fn build_status_lines(&self, locale: &Locale, i18n: &I18n) -> String {
        let condensed_line = i18n.t(
            *locale,
            "footer.condensed",
            &[("model", &self.condensed_model)],
        );
        let segmented_line = i18n.t(
            *locale,
            "footer.segmented",
            &[("model", &self.segmented_model)],
        );
        let check_line = self.format_check_line(locale, i18n);
        format!("{}\n{}\n{}", condensed_line, segmented_line, check_line)
    }

    fn format_check_line(&self, locale: &Locale, i18n: &I18n) -> String {
        let check = &self.check;
        if check.model.is_empty() {
            return i18n.t(*locale, "footer.check_not_configured", &[]);
        }
        if check.attempted && check.succeeded && check.backup_used {
            return i18n.t(
                *locale,
                "footer.check_backup_success",
                &[
                    ("model", &check.model),
                    ("backup_model", &check.backup_model),
                ],
            );
        }
        if check.attempted && check.failed && check.backup_used {
            return i18n.t(
                *locale,
                "footer.check_backup_failed",
                &[
                    ("model", &check.model),
                    ("backup_model", &check.backup_model),
                ],
            );
        }
        if check.attempted && check.failed {
            return i18n.t(*locale, "footer.check_failed", &[("model", &check.model)]);
        }
        if check.attempted && check.succeeded {
            return i18n.t(*locale, "footer.check_success", &[("model", &check.model)]);
        }
        // Not attempted = not triggered
        i18n.t(
            *locale,
            "footer.check_not_triggered",
            &[("model", &check.model)],
        )
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
    /// Execution trace with model names and check model status.
    pub trace: RecapTrace,
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

fn user_message(content: String) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
        content: ChatCompletionRequestUserMessageContent::Text(content),
        name: None,
    })
}

fn chunked_history_notice(locale: &Locale) -> &'static str {
    match locale {
        Locale::ZhHans => {
            "聊天记录会在后续 user 消息中按顺序提供，每个消息块最多包含 20 条聊天消息。"
        }
        Locale::ZhHant => {
            "聊天記錄會在後續 user 訊息中按順序提供，每個訊息區塊最多包含 20 條聊天訊息。"
        }
        Locale::En => {
            "The chat history is provided in following user messages in chronological order, with up to 20 chat messages per block."
        }
    }
}

fn append_chat_history_chunks(
    messages: &mut Vec<ChatCompletionRequestMessage>,
    chunks: &[String],
    _locale: Option<&Locale>,
) {
    for chunk in chunks {
        messages.push(user_message(chunk.clone()));
    }
}

fn format_message_line(h: &ChatHistory) -> Option<String> {
    if h.text.is_empty() {
        return None;
    }
    let sender = format_user_name(&h.from_full_name, &h.from_username);
    Some(format!(
        "msgId:{}: {} sent: {}",
        h.message_id, sender, h.text
    ))
}

fn format_message_chunks(history: &[ChatHistory], chunk_size: usize) -> Vec<String> {
    let chunk_size = chunk_size.max(1);
    let mut chunks = Vec::new();
    let mut current = Vec::with_capacity(chunk_size);

    for line in history.iter().filter_map(format_message_line) {
        current.push(line);
        if current.len() == chunk_size {
            chunks.push(current.join("\n"));
            current.clear();
        }
    }

    if !current.is_empty() {
        chunks.push(current.join("\n"));
    }

    chunks
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
                chat_cid,
                topic.since_id,
                escape_html(&topic.topic_name)
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
        output.push(format!(
            "{}{}{}",
            participants_label, colon, participants_str
        ));

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

#[cfg(test)]
mod tests {
    use super::*;

    fn chat_history(message_id: i64, text: &str) -> ChatHistory {
        ChatHistory {
            id: message_id,
            chat_id: 1,
            message_id,
            from_id: 100,
            from_full_name: "Alice Example".to_string(),
            from_username: "alice".to_string(),
            kind: "text".to_string(),
            text: text.to_string(),
            media_url: String::new(),
            created_at: message_id,
        }
    }

    #[test]
    fn format_message_chunks_groups_twenty_text_messages() {
        let history = (1..=41)
            .map(|id| chat_history(id, &format!("message {id}")))
            .collect::<Vec<_>>();

        let chunks = format_message_chunks(&history, CHAT_MESSAGES_PER_USER_PROMPT);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].lines().count(), 20);
        assert_eq!(chunks[1].lines().count(), 20);
        assert_eq!(chunks[2].lines().count(), 1);
        assert!(chunks[0].contains("msgId:1: alice sent: message 1"));
        assert!(chunks[2].contains("msgId:41: alice sent: message 41"));
    }

    #[test]
    fn format_message_chunks_skips_empty_text_messages() {
        let history = vec![
            chat_history(1, "first"),
            chat_history(2, ""),
            chat_history(3, "third"),
        ];

        let chunks = format_message_chunks(&history, 20);

        assert_eq!(
            chunks,
            vec!["msgId:1: alice sent: first\nmsgId:3: alice sent: third"]
        );
    }
}
