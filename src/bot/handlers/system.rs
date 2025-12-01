use std::sync::Arc;

use teloxide::{prelude::*, utils::command::BotCommands};

use crate::bot::{commands::Command, context::AppContext};

pub struct SystemHandlers;

impl SystemHandlers {
    pub async fn handle_start(bot: Bot, msg: Message, ctx: Arc<AppContext>) -> ResponseResult<()> {
        let text = ctx.i18n.t(ctx.config.locale, "bot.start", &[]);
        bot.send_message(msg.chat.id, text).await?;
        Ok(())
    }

    pub async fn handle_help(bot: Bot, msg: Message, ctx: Arc<AppContext>) -> ResponseResult<()> {
        let mut help = String::from("Commands:\n");
        help.push_str(&Command::descriptions().to_string());
        let text = ctx.i18n.t(ctx.config.locale, "bot.help", &[]) + "\n\n" + &help;
        bot.send_message(msg.chat.id, text).await?;
        Ok(())
    }

    pub async fn handle_cancel(bot: Bot, msg: Message, ctx: Arc<AppContext>) -> ResponseResult<()> {
        let text = ctx.i18n.t(ctx.config.locale, "bot.cancel", &[]);
        bot.send_message(msg.chat.id, text).await?;
        Ok(())
    }
}
