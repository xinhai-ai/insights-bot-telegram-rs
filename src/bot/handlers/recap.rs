use std::sync::Arc;

use teloxide::{
    prelude::*,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup},
};
use tracing::{error, warn};

use crate::{
    bot::context::AppContext,
    services::{rate_limit::RateKey, recap::RecapService},
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
        let chat_id = msg.chat.id;
        if !msg.chat.is_group() && !msg.chat.is_supergroup() {
            bot.send_message(chat_id, "此指令僅適用於群組。").await?;
            return Ok(());
        }

        // Best-effort admin check
        if let Some(from) = msg.from() {
            if let Err(err) = bot.get_chat_member(chat_id, from.id).await {
                warn!("admin check skipped: {err:?}");
            }
        }

        let cfg = match crate::db::recap_config::get_or_create_recap_config(&ctx.db.pool, chat_id.0)
            .await
        {
            Ok(c) => c,
            Err(err) => {
                error!("load recap config failed: {err:?}");
                bot.send_message(chat_id, "載入設定失敗，稍後再試。")
                    .await?;
                return Ok(());
            }
        };

        let text = format!(
            "Recap設定\n啟用: {}\n自動 recap: {}\n每天次數: {}\n\n請點按鈕調整。",
            if cfg.enabled { "開" } else { "關" },
            if cfg.auto_recap_enabled { "開" } else { "關" },
            cfg.auto_recap_rates_per_day.unwrap_or(1)
        );

        let kb = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("Recap 開", "cfg:enable:on"),
                InlineKeyboardButton::callback("Recap 關", "cfg:enable:off"),
            ],
            vec![
                InlineKeyboardButton::callback("Auto 開", "cfg:auto:on"),
                InlineKeyboardButton::callback("Auto 關", "cfg:auto:off"),
            ],
            vec![
                InlineKeyboardButton::callback("每天 1 次", "cfg:rate:1"),
                InlineKeyboardButton::callback("每天 2 次", "cfg:rate:2"),
                InlineKeyboardButton::callback("每天 4 次", "cfg:rate:4"),
            ],
        ]);

        bot.send_message(chat_id, text).reply_markup(kb).await?;
        Ok(())
    }

    pub async fn handle_callback_query(
        bot: Bot,
        q: CallbackQuery,
        ctx: Arc<AppContext>,
    ) -> ResponseResult<()> {
        let id = q.id.clone();
        bot.answer_callback_query(id).await?;
        let Some(msg) = q.message else {
            return Ok(());
        };
        let chat_id = msg.chat.id;
        let data = q.data.clone().unwrap_or_default();
        let parts: Vec<&str> = data.split(':').collect();
        if parts.len() < 3 || parts[0] != "cfg" {
            return Ok(());
        }

        match (parts[1], parts[2]) {
            ("enable", "on") => {
                crate::db::recap_config::set_enabled(&ctx.db.pool, chat_id.0, true)
                    .await
                    .map_err(|e| error!("set enable failed: {e:?}"))
                    .ok();
            }
            ("enable", "off") => {
                crate::db::recap_config::set_enabled(&ctx.db.pool, chat_id.0, false)
                    .await
                    .map_err(|e| error!("set enable failed: {e:?}"))
                    .ok();
            }
            ("auto", "on") => {
                crate::db::recap_config::set_auto_recap(&ctx.db.pool, chat_id.0, true)
                    .await
                    .map_err(|e| error!("set auto failed: {e:?}"))
                    .ok();
            }
            ("auto", "off") => {
                crate::db::recap_config::set_auto_recap(&ctx.db.pool, chat_id.0, false)
                    .await
                    .map_err(|e| error!("set auto failed: {e:?}"))
                    .ok();
            }
            ("rate", v) => {
                if let Ok(n) = v.parse::<i32>() {
                    crate::db::recap_config::set_rates_per_day(&ctx.db.pool, chat_id.0, n)
                        .await
                        .map_err(|e| error!("set rate failed: {e:?}"))
                        .ok();
                }
            }
            _ => {}
        }

        bot.send_message(chat_id, "設定已更新。").await.ok();
        Ok(())
    }
}
