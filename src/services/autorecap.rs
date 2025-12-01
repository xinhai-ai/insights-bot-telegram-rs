use std::{sync::Arc, time::Duration};

use crate::db::models::RecapConfig;
use chrono::Utc;
use teloxide::prelude::*;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::bot::handlers::recap::build_recap_nodes;
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
        match service
            .recap_chat_dual(cfg.chat_id, 200, &ctx.config.locale, &ctx.i18n)
            .await
        {
            Ok(output) => {
                // Build Telegraph page title.
                let chat_title = extract_chat_title(&cfg);
                let page_title = ctx.i18n.t(
                    ctx.config.locale,
                    "recap.auto_page_title",
                    &[("chat", &chat_title)],
                );

                let nodes = build_recap_nodes(
                    &output.condensed_summary,
                    &output.segmented_summary,
                    &output.condensed_model,
                    &output.segmented_model,
                    cfg.chat_id,
                    &ctx.config.locale,
                    &ctx.i18n,
                );

                let telegraph_url = if let Some(ref telegraph) = ctx.telegraph {
                    match telegraph.create_page_auto_nodes(&page_title, &nodes).await {
                        Ok(urls) => urls.first().cloned(),
                        Err(err) => {
                            warn!("telegraph page creation failed (auto_recap): {err:?}");
                            None
                        }
                    }
                } else {
                    None
                };

                // Escape HTML for message content
                fn escape_html(text: &str) -> String {
                    text.replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;")
                }

                // Format outbound message.
                let final_text = if let Some(url) = telegraph_url {
                    ctx.i18n.t(
                        ctx.config.locale,
                        "recap.auto_published",
                        &[
                            ("url", &url),
                            ("title", &escape_html(&page_title)),
                            ("condensed", &escape_html(&output.condensed_summary)),
                            ("group", &chat_title),
                            ("condensed_model", &output.condensed_model),
                            ("segmented_model", &output.segmented_model),
                        ],
                    )
                } else {
                    ctx.i18n.t(
                        ctx.config.locale,
                        "recap.auto_no_telegraph",
                        &[
                            ("condensed", &escape_html(&output.condensed_summary)),
                            ("segmented", &escape_html(&output.segmented_summary)),
                            ("group", &chat_title),
                            ("condensed_model", &output.condensed_model),
                            ("segmented_model", &output.segmented_model),
                        ],
                    )
                };

                let msg_body = final_text.clone();
                if let Err(err) = bot
                    .send_message(ChatId(cfg.chat_id), msg_body.clone())
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await
                {
                    warn!("auto_recap send to chat {} failed: {err:?}", cfg.chat_id);
                }

                // send to subscribers best-effort
                match crate::db::recap_config::list_subscribers(&ctx.db.pool, cfg.chat_id).await {
                    Ok(subs) => {
                        for sub in subs {
                            if let Err(err) = bot
                                .send_message(ChatId(sub.user_id), msg_body.clone())
                                .parse_mode(teloxide::types::ParseMode::Html)
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

fn extract_chat_title(cfg: &RecapConfig) -> String {
    cfg.chat_id.to_string()
}
