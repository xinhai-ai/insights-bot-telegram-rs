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
            bot.send_message(msg.chat.id, "請在群組內使用此指令訂閱該群的 recap。").await?;
            return Ok(());
        }

        let svc = RecapService::new(&ctx.db, &ctx.openai);
        let user_id = msg.from().map(|u| u.id.0);
        if let Some(uid) = user_id {
            match svc.subscribe(msg.chat.id.0, uid as i64).await {
                Ok(_) => {
                    bot.send_message(msg.chat.id, "已訂閱本群組 recap。").await?;
                }
                Err(err) => {
                    error!("subscribe recap failed: {err:?}");
                    bot.send_message(msg.chat.id, "訂閱失敗，稍後再試。").await?;
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
        if msg.chat.is_private() {
            bot.send_message(msg.chat.id, "請在群組內使用此指令取消該群的 recap 訂閱。").await?;
            return Ok(());
        }

        let svc = RecapService::new(&ctx.db, &ctx.openai);
        let user_id = msg.from().map(|u| u.id.0);
        if let Some(uid) = user_id {
            if let Err(err) = svc.unsubscribe(msg.chat.id.0, uid as i64).await {
                error!("unsubscribe recap failed: {err:?}");
                bot.send_message(msg.chat.id, "取消訂閱失敗，稍後再試。").await?;
            } else {
                bot.send_message(msg.chat.id, "已取消訂閱本群組 recap。").await?;
            }
        }
        Ok(())
    }
}
