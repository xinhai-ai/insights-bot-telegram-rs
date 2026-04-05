use std::sync::Arc;

use teloxide::{prelude::*, utils::command::BotCommands};

use crate::bot::{commands::Command, context::AppContext};

pub struct SystemHandlers;

impl SystemHandlers {
    pub async fn handle_start(bot: Bot, msg: Message, ctx: Arc<AppContext>) -> ResponseResult<()> {
        let greeting = ctx.i18n.t(ctx.config.locale, "bot.start", &[]);
        let help = format!("{}\n\n{}", greeting, command_list());
        bot.send_message(msg.chat.id, help).await?;
        Ok(())
    }

    pub async fn handle_help(bot: Bot, msg: Message, ctx: Arc<AppContext>) -> ResponseResult<()> {
        let header = ctx.i18n.t(ctx.config.locale, "bot.help", &[]);
        let text = format!("{header}\n\n{}", command_list());
        bot.send_message(msg.chat.id, text).await?;
        Ok(())
    }

    pub async fn handle_cancel(bot: Bot, msg: Message, ctx: Arc<AppContext>) -> ResponseResult<()> {
        let text = ctx.i18n.t(ctx.config.locale, "bot.cancel", &[]);
        bot.send_message(msg.chat.id, text).await?;
        Ok(())
    }
}

fn command_list() -> String {
    format!("Commands:\n{}", Command::descriptions())
}
