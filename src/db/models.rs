use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Chat {
    pub id: i64,
    pub title: Option<String>,
    pub username: Option<String>,
    pub kind: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageKind {
    Text,
    Photo,
    Video,
    Audio,
    Voice,
    Document,
    Sticker,
    Other,
}

impl MessageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageKind::Text => "text",
            MessageKind::Photo => "photo",
            MessageKind::Video => "video",
            MessageKind::Audio => "audio",
            MessageKind::Voice => "voice",
            MessageKind::Document => "document",
            MessageKind::Sticker => "sticker",
            MessageKind::Other => "other",
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatHistory {
    pub id: i64,
    pub chat_id: i64,
    pub message_id: i64,
    pub from_id: Option<i64>,
    /// Empty string when NULL (due to SQLx Any driver limitation).
    pub from_full_name: String,
    /// Empty string when NULL (due to SQLx Any driver limitation).
    pub from_username: String,
    pub kind: String,
    /// Empty string when NULL (due to SQLx Any driver limitation).
    pub text: String,
    /// Empty string when NULL (due to SQLx Any driver limitation).
    pub media_url: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RecapConfig {
    pub chat_id: i64,
    pub enabled: bool,
    pub auto_recap_enabled: bool,
    pub last_recap_at: Option<i64>,
    pub updated_at: Option<i64>,
    /// Auto-recap frequency: 2, 3, or 4 times per day.
    pub auto_recap_rates_per_day: i32,
    /// Whether to pin auto-recap messages in the group.
    pub pin_auto_recap_message: bool,
    /// Message ID of the last pinned recap (for unpin-before-pin).
    pub last_pinned_message_id: Option<i64>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RecapLog {
    pub id: String,
    pub chat_id: i64,
    pub prompt: Option<String>,
    pub recap_text: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub created_at: Option<i64>,
}
