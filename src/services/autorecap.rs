use std::{sync::Arc, time::Duration};

use chrono::Utc;
use teloxide::prelude::*;
use tokio::time::interval;
use tracing::{error, info, warn};

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

    for cfg in configs {
        match service.recap_chat(cfg.chat_id, 200).await {
            Ok(recap) => {
                if let Err(err) = bot
                    .send_message(ChatId(cfg.chat_id), recap.text.clone())
                    .await
                {
                    warn!("auto_recap send to chat {} failed: {err:?}", cfg.chat_id);
                }

                // send to subscribers best-effort
                match crate::db::recap_config::list_subscribers(&ctx.db.pool, cfg.chat_id).await {
                    Ok(subs) => {
                        for sub in subs {
                            if let Err(err) = bot
                                .send_message(ChatId(sub.user_id), recap.text.clone())
                                .await
                            {
                                warn!(
                                    "auto_recap send to subscriber {} failed: {err:?}",
                                    sub.user_id
                                );
                            }
                        }
                    }
                    Err(err) => warn!("list_subscribers failed: {err:?}"),
                }

                if let Err(err) = crate::db::recap_config::upsert_recap_config(
                    &ctx.db.pool,
                    &crate::db::models::RecapConfig {
                        last_recap_at: Some(now),
                        ..cfg
                    },
                )
                .await
                {
                    warn!("update last_recap_at failed: {err:?}");
                }
            }
            Err(err) => warn!("auto_recap for chat {} failed: {err:?}", cfg.chat_id),
        }
    }

    info!("auto_recap completed");
    Ok(())
}
