use std::sync::Arc;

use teloxide::prelude::*;
use tracing::error;

use crate::{bot::context::AppContext, services::recap::RecapService};

pub struct SubscribeHandlers;

impl SubscribeHandlers {
    pub async fn handle_subscribe(
        bot: Bot,
        msg: Message,
        ctx: Arc<AppContext>,
    ) -> ResponseResult<()> {
        let svc = RecapService::new(&ctx.db, &ctx.openai);
        let user_id = msg.from().map(|u| u.id.0);
        if let Some(uid) = user_id {
            match svc.subscribe(msg.chat.id.0, uid as i64).await {
                Ok(_) => {
                    let text = ctx.i18n.t(ctx.config.locale, "recap.subscribe.ok", &[]);
                    bot.send_message(msg.chat.id, text).await?;
                }
                Err(err) => {
                    error!("subscribe recap failed: {err:?}");
                    bot.send_message(msg.chat.id, "Subscribe failed.").await?;
                }
            }
        } else {
            bot.send_message(msg.chat.id, "需要識別使用者。").await?;
        }
        Ok(())
    }

    pub async fn handle_unsubscribe(
        bot: Bot,
        msg: Message,
        ctx: Arc<AppContext>,
    ) -> ResponseResult<()> {
        let svc = RecapService::new(&ctx.db, &ctx.openai);
        let user_id = msg.from().map(|u| u.id.0);
        if let Some(uid) = user_id {
            if let Err(err) = svc.unsubscribe(msg.chat.id.0, uid as i64).await {
                error!("unsubscribe recap failed: {err:?}");
                bot.send_message(msg.chat.id, "Unsubscribe failed.").await?;
            } else {
                let text = ctx.i18n.t(ctx.config.locale, "recap.unsubscribe.ok", &[]);
                bot.send_message(msg.chat.id, text).await?;
            }
        }
        Ok(())
    }
}
