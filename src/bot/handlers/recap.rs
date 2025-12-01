use std::sync::Arc;

use teloxide::{prelude::*, types::CallbackQuery};
use tracing::error;

use crate::{
    bot::context::AppContext, services::rate_limit::RateKey, services::recap::RecapService,
};

pub struct RecapHandlers;

impl RecapHandlers {
    pub async fn handle_recap(bot: Bot, msg: Message, ctx: Arc<AppContext>) -> ResponseResult<()> {
        if let Err(err) = ctx.limiter.check(RateKey(msg.chat.id.0, "recap")) {
            bot.send_message(msg.chat.id, "Too many recap requests, please slow down.")
                .await?;
            error!("rate limited recap: {err:?}");
            return Ok(());
        }
        let svc = RecapService::new(&ctx.db, &ctx.openai);
        match svc.recap_chat(msg.chat.id.0, 200).await {
            Ok(res) => {
                bot.send_message(msg.chat.id, res.text).await?;
            }
            Err(err) => {
                error!("recap failed: {err:?}");
                bot.send_message(msg.chat.id, "Recap failed, please try later.")
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn handle_configure_recap(
        bot: Bot,
        msg: Message,
        ctx: Arc<AppContext>,
    ) -> ResponseResult<()> {
        let text = ctx
            .i18n
            .t(ctx.config.locale, "recap.configure.placeholder", &[]);
        bot.send_message(msg.chat.id, text).await?;
        Ok(())
    }

    pub async fn handle_callback_query(
        bot: Bot,
        q: CallbackQuery,
        ctx: Arc<AppContext>,
    ) -> ResponseResult<()> {
        let id = q.id.clone();
        bot.answer_callback_query(id).await?;
        if let Some(msg) = q.message {
            let text = ctx
                .i18n
                .t(ctx.config.locale, "recap.configure.not_implemented", &[]);
            bot.send_message(msg.chat.id, text).await?;
        }
        Ok(())
    }
}
