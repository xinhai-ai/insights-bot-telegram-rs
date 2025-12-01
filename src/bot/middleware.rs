use std::sync::Arc;

use teloxide::{dispatching::UpdateHandler, dptree, types::Message};
use tracing::warn;

use crate::{bot::context::AppContext, db::chat_history, db::models::MessageKind};

pub fn record_message() -> UpdateHandler<anyhow::Error> {
    dptree::endpoint(|msg: Message, ctx: Arc<AppContext>| async move {
        let kind = classify_message(&msg);
        let text = msg.text().map(|s| s.to_string());
        let media_url = None; // placeholder: future media download
        let created_at = msg.date.timestamp();

        if let Err(err) = chat_history::insert_message(
            &ctx.db.pool,
            msg.chat.id.0,
            msg.id.0 as i64,
            msg.from().map(|u| u.id.0 as i64),
            msg.from().and_then(|u| u.username.clone()),
            kind,
            text,
            media_url,
            created_at,
        )
        .await
        {
            warn!("record_message failed: {err:?}");
        }

        Ok::<(), anyhow::Error>(())
    })
}

fn classify_message(msg: &Message) -> MessageKind {
    if msg.text().is_some() {
        return MessageKind::Text;
    }
    MessageKind::Other
}
