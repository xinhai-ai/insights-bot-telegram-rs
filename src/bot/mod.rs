pub mod commands;
pub mod context;
pub mod handlers;
pub mod middleware;
pub mod router;

use std::sync::Arc;

use anyhow::Result;
use teloxide::prelude::*;
use tracing::info;

use context::AppContext;

pub async fn run(ctx: Arc<AppContext>) -> Result<()> {
    let bot = Bot::new(&ctx.config.telegram.bot_token);
    let mut dispatcher = router::build_dispatcher(bot, ctx.clone());

    info!("starting telegram dispatcher");
    dispatcher.dispatch().await;
    Ok(())
}
