use std::sync::Arc;

use teloxide::prelude::*;
use tracing::warn;

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
        let Some(from) = msg.from() else {
            let text = ctx
                .i18n
                .t(ctx.config.locale, "recap.forwarded.unable_to_identify", &[]);
            bot.send_message(msg.chat.id, text).await?;
            return Ok(());
        };

        if let Err(err) = ctx.limiter.check(crate::services::rate_limit::RateKey(
            msg.chat.id.0,
            "recap_forwarded",
        )) {
            let text = ctx.i18n.t(ctx.config.locale, "recap.rate_limited", &[]);
            bot.send_message(msg.chat.id, text).await?;
            warn!("rate limited recap_forwarded: {err:?}");
            return Ok(());
        }

        match svc.recap_forwarded(from.id.0 as i64, 200, &ctx.i18n).await {
            Ok(res) => {
                bot.send_message(msg.chat.id, res.text).await?;
            }
            Err(err) => {
                warn!("recap forwarded failed: {err:?}");
                let text = ctx
                    .i18n
                    .t(ctx.config.locale, "recap.forwarded.no_messages", &[]);
                bot.send_message(msg.chat.id, text).await?;
            }
        }
        Ok(())
    }
}
