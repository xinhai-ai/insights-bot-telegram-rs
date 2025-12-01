use std::sync::Arc;

use teloxide::types::{ChatKind, Message};
use tracing::{debug, warn};

use crate::{bot::context::AppContext, db::chat_history, db::models::MessageKind};

/// Record a message to the database. Called from the router as a side effect.
pub async fn record_message(ctx: Arc<AppContext>, msg: Message) {
    let text = msg
        .text()
        .map(|s| s.to_string())
        .or_else(|| msg.caption().map(|s| s.to_string()));

    let kind = if text.is_some() {
        MessageKind::Text
    } else if msg.photo().is_some() {
        MessageKind::Photo
    } else if msg.video().is_some() {
        MessageKind::Video
    } else if msg.audio().is_some() {
        MessageKind::Audio
    } else if msg.voice().is_some() {
        MessageKind::Voice
    } else if msg.document().is_some() {
        MessageKind::Document
    } else if msg.sticker().is_some() {
        MessageKind::Sticker
    } else {
        MessageKind::Other
    };

    let created_at = msg.date.timestamp();
    let chat_id = msg.chat.id.0;
    let from_id = msg.from().map(|u| u.id.0 as i64);
    // Extract full name (first_name + last_name)
    let from_full_name = msg.from().map(|u| {
        let mut name = u.first_name.clone();
        if let Some(ref last) = u.last_name {
            name.push(' ');
            name.push_str(last);
        }
        name
    });
    let from_username = msg.from().and_then(|u| u.username.clone());

    let preview = text.clone().unwrap_or_else(|| "<non-text>".to_string());
    let from = msg
        .from()
        .map(|u| u.username.clone().unwrap_or_else(|| u.first_name.clone()))
        .unwrap_or_else(|| "<unknown>".to_string());

    debug!(
        chat_id = chat_id,
        message_id = msg.id.0,
        kind = ?kind,
        from = %from,
        preview = %preview,
        "recording message"
    );

    // Record to chat_histories
    if let Err(err) = chat_history::insert_message(
        &ctx.db.pool,
        chat_id,
        msg.id.0 as i64,
        from_id,
        from_full_name,
        from_username,
        kind,
        text.clone(),
        None,
        created_at,
    )
    .await
    {
        warn!("record_message failed: {err:?}");
    }

    // For private chats with forwarded messages, also record to forwarded_histories
    let is_private = matches!(msg.chat.kind, ChatKind::Private(_));
    let is_forwarded = msg.forward().is_some();
    if is_private && is_forwarded && text.is_some() {
        if let Some(uid) = msg.from().map(|u| u.id.0 as i64) {
            if let Err(err) = chat_history::insert_forwarded(
                &ctx.db.pool,
                uid,
                None,
                None,
                MessageKind::Text,
                text,
                created_at,
            )
            .await
            {
                warn!("record_forwarded failed: {err:?}");
            }
        }
    }
}
