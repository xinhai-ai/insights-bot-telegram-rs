use std::sync::Arc;

use teloxide::{
    dispatching::UpdateHandler,
    dptree,
    types::{ChatKind, Message},
    RequestError,
};
use tracing::warn;

use crate::{bot::context::AppContext, db::chat_history, db::models::MessageKind};

pub fn record_message() -> UpdateHandler<RequestError> {
    dptree::filter_map(|ctx: Arc<AppContext>, msg: Message| {
        let msg_clone = msg.clone();
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            let text = msg_clone
                .text()
                .map(|s| s.to_string())
                .or_else(|| msg_clone.caption().map(|s| s.to_string()));
            let kind = if text.is_some() {
                MessageKind::Text
            } else {
                MessageKind::Other
            };
            let created_at = msg_clone.date.timestamp();
            let chat_id = msg_clone.chat.id.0;
            let from_id = msg_clone.from().map(|u| u.id.0 as i64);
            let from_username = msg_clone.from().and_then(|u| u.username.clone());

            if let Err(err) = chat_history::insert_message(
                &ctx_clone.db.pool,
                chat_id,
                msg_clone.id.0 as i64,
                from_id,
                from_username.clone(),
                kind.clone(),
                text.clone(),
                None,
                created_at,
            )
            .await
            {
                warn!("record_message failed: {err:?}");
            }

            let is_private = matches!(msg_clone.chat.kind, ChatKind::Private(_));
            let is_forwarded = msg_clone.forward().is_some();
            if is_private && is_forwarded && text.is_some() {
                if let Some(uid) = msg_clone.from().map(|u| u.id.0 as i64) {
                    if let Err(err) = chat_history::insert_forwarded(
                        &ctx_clone.db.pool,
                        uid,
                        None,
                        None,
                        MessageKind::Text,
                        text.clone(),
                        created_at,
                    )
                    .await
                    {
                        warn!("record_forwarded failed: {err:?}");
                    }
                }
            }
        });

        Some(msg)
    })
}
