use insights_bot_telegram_rs::{
    bot::commands::Command,
    db::{chat_history, models::RecapConfig, recap_config},
};
use sqlx::{AnyPool, any::AnyPoolOptions};
use teloxide::utils::command::BotCommands;

async fn setup_sqlite_pool() -> AnyPool {
    sqlx::any::install_default_drivers();
    let pool = AnyPoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("sqlite memory pool");

    for statement in include_str!("../migrations/sqlite/0001_init.sql").split(';') {
        let stmt: String = statement
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with("--")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        sqlx::query(stmt)
            .execute(&pool)
            .await
            .expect("migration statement should run");
    }

    pool
}

#[test]
fn command_surface_only_lists_supported_commands() {
    let descriptions = Command::descriptions().to_string();
    for supported in ["/start", "/help", "/cancel", "/recap", "/configure_recap"] {
        assert!(
            descriptions.contains(supported),
            "missing supported command {supported}"
        );
    }

    for removed in [
        "/subscribe_recap",
        "/unsubscribe_recap",
        "/recap_forwarded_start",
        "/recap_forwarded",
    ] {
        assert!(
            !descriptions.contains(removed),
            "removed command {removed} should not be listed"
        );
    }
}

#[tokio::test]
async fn disabled_chat_blocks_message_capture_and_auto_recap_due_selection() {
    let pool = setup_sqlite_pool().await;

    let cfg = RecapConfig {
        chat_id: 42,
        enabled: false,
        auto_recap_enabled: true,
        last_recap_at: None,
        updated_at: None,
    };
    recap_config::upsert_recap_config(&pool, &cfg)
        .await
        .expect("config insert should succeed");

    assert!(
        !chat_history::is_recap_enabled(&pool, 42)
            .await
            .expect("enablement lookup should succeed")
    );

    let due = recap_config::list_due_for_auto_recap(&pool, i64::MAX)
        .await
        .expect("due lookup should succeed");
    assert!(
        due.iter().all(|item| item.chat_id != 42),
        "disabled chat must not be due for auto recap"
    );
}

#[tokio::test]
async fn due_query_respects_fixed_cutoff_timestamp() {
    let pool = setup_sqlite_pool().await;

    recap_config::upsert_recap_config(
        &pool,
        &RecapConfig {
            chat_id: 1,
            enabled: true,
            auto_recap_enabled: true,
            last_recap_at: Some(100),
            updated_at: None,
        },
    )
    .await
    .expect("old config insert should succeed");

    recap_config::upsert_recap_config(
        &pool,
        &RecapConfig {
            chat_id: 2,
            enabled: true,
            auto_recap_enabled: true,
            last_recap_at: Some(900),
            updated_at: None,
        },
    )
    .await
    .expect("recent config insert should succeed");

    recap_config::upsert_recap_config(
        &pool,
        &RecapConfig {
            chat_id: 3,
            enabled: true,
            auto_recap_enabled: true,
            last_recap_at: None,
            updated_at: None,
        },
    )
    .await
    .expect("never-run config insert should succeed");

    let due = recap_config::list_due_for_auto_recap(&pool, 500)
        .await
        .expect("due lookup should succeed");
    let due_chat_ids: Vec<i64> = due.into_iter().map(|item| item.chat_id).collect();

    assert!(due_chat_ids.contains(&1), "older chat should be due");
    assert!(due_chat_ids.contains(&3), "never-run chat should be due");
    assert!(
        !due_chat_ids.contains(&2),
        "chat newer than the fixed cutoff must not be due"
    );
}
