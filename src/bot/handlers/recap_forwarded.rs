use std::sync::Arc;

use teloxide::prelude::*;
use tracing::error;

use crate::{bot::context::AppContext, services::recap::RecapService};

pub struct RecapForwardedHandlers;

impl RecapForwardedHandlers {
    pub async fn handle_start_forwarded(
        bot: Bot,
        msg: Message,
        ctx: Arc<AppContext>,
    ) -> ResponseResult<()> {
        let text = ctx.i18n.t(ctx.config.locale, "recap.forwarded.start", &[]);
        bot.send_message(msg.chat.id, text).await?;
        Ok(())
    }

    pub async fn handle_forwarded(
        bot: Bot,
        msg: Message,
        ctx: Arc<AppContext>,
    ) -> ResponseResult<()> {
        let svc = RecapService::new(&ctx.db, &ctx.openai);
        match msg.from() {
            Some(from) => match svc.recap_forwarded(from.id.0 as i64, 200).await {
                Ok(res) => {
                    bot.send_message(msg.chat.id, res.text).await?;
                }
                Err(err) => {
                    error!("recap forwarded failed: {err:?}");
                    bot.send_message(msg.chat.id, "Recap forwarded failed.")
                        .await?;
                }
            },
            None => {
                bot.send_message(msg.chat.id, "无法识别用户。").await?;
            }
        }
        Ok(())
    }
}
