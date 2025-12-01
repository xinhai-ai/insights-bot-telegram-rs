use std::{sync::Arc, time::Duration};

use chrono::Utc;
use teloxide::prelude::*;
use tokio::time::interval;
use tracing::{error, info};

use crate::{bot::context::AppContext, services::recap::RecapService};

pub async fn spawn_autorecap(ctx: Arc<AppContext>) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            if let Err(err) = run_once(ctx.clone()).await {
                error!("auto_recap tick failed: {err:?}");
            }
        }
    });
}

async fn run_once(ctx: Arc<AppContext>) -> anyhow::Result<()> {
    let now = Utc::now().timestamp();
    let configs = crate::db::recap_config::list_due_for_auto_recap(&ctx.db.pool, now).await?;
    if configs.is_empty() {
        return Ok(());
    }

    let bot = Bot::new(&ctx.config.telegram.bot_token);
    let service = RecapService::new(&ctx.db, &ctx.openai);

    for cfg in &configs {
        let recap = service.recap_chat(cfg.chat_id, 200).await?;
        bot.send_message(ChatId(cfg.chat_id), recap.text.clone())
            .await?;
        crate::db::recap_config::upsert_recap_config(
            &ctx.db.pool,
            &crate::db::models::RecapConfig {
                last_recap_at: Some(now),
                ..cfg.clone()
            },
        )
        .await?;
    }

    info!("auto_recap completed for {} chats", configs.len());
    Ok(())
}
