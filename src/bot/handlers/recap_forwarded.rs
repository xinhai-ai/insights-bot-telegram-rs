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
            bot.send_message(msg.chat.id, "無法識別使用者。").await?;
            return Ok(());
        };

        if let Err(err) = ctx.limiter.check(crate::services::rate_limit::RateKey(
            msg.chat.id.0,
            "recap_forwarded",
        )) {
            bot.send_message(msg.chat.id, "Too many requests, please try later.")
                .await?;
            warn!("rate limited recap_forwarded: {err:?}");
            return Ok(());
        }

        match svc.recap_forwarded(from.id.0 as i64, 200).await {
            Ok(res) => {
                bot.send_message(msg.chat.id, res.text).await?;
            }
            Err(err) => {
                warn!("recap forwarded failed: {err:?}");
                bot.send_message(
                    msg.chat.id,
                    "目前沒有可用的轉發訊息，請先用 /recap_forwarded_start 並轉發訊息。",
                )
                .await?;
            }
        }
        Ok(())
    }
}
