use std::env;

use insights_bot_telegram_rs::config::AppConfig;

#[test]
fn config_from_env_reads_required() {
    // set required vars (env mutation is unsafe in Rust 2024)
    unsafe {
        env::set_var("TELEGRAM_BOT_TOKEN", "token");
        env::set_var("OPENAI_API_SECRET", "key");
        env::set_var("DATABASE_URL", "sqlite::memory:");
        env::remove_var("INSIGHTS_LANG");
    }

    let cfg = AppConfig::from_env().expect("config should load");
    assert_eq!(cfg.telegram.bot_token, "token");
    assert_eq!(cfg.openai.api_key, "key");
    // default locale should be en
    assert_eq!(cfg.locale.code(), "en");
}
