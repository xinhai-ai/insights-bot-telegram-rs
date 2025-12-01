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
        if msg.chat.is_private() {
            bot.send_message(
                msg.chat.id,
                "Please use this command in a group to subscribe to its recaps.",
            )
            .await?;
            return Ok(());
        }

        let svc = RecapService::new(&ctx.db, &ctx.openai);
        let user_id = msg.from().map(|u| u.id.0);
        if let Some(uid) = user_id {
            match svc.subscribe(msg.chat.id.0, uid as i64).await {
                Ok(_) => {
                    bot.send_message(msg.chat.id, "Subscribed to this group's recaps.")
                        .await?;
                }
                Err(err) => {
                    error!("subscribe recap failed: {err:?}");
                    bot.send_message(msg.chat.id, "Subscription failed, please try later.")
                        .await?;
                }
            }
        } else {
            bot.send_message(msg.chat.id, "Unable to identify user.")
                .await?;
        }
        Ok(())
    }

    pub async fn handle_unsubscribe(
        bot: Bot,
        msg: Message,
        ctx: Arc<AppContext>,
    ) -> ResponseResult<()> {
        if msg.chat.is_private() {
            bot.send_message(
                msg.chat.id,
                "Please use this command in a group to unsubscribe from its recaps.",
            )
            .await?;
            return Ok(());
        }

        let svc = RecapService::new(&ctx.db, &ctx.openai);
        let user_id = msg.from().map(|u| u.id.0);
        if let Some(uid) = user_id {
            if let Err(err) = svc.unsubscribe(msg.chat.id.0, uid as i64).await {
                error!("unsubscribe recap failed: {err:?}");
                bot.send_message(msg.chat.id, "Unsubscribe failed, please try later.")
                    .await?;
            } else {
                bot.send_message(msg.chat.id, "Unsubscribed from this group's recaps.")
                    .await?;
            }
        }
        Ok(())
    }
}
