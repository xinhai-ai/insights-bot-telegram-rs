use std::env;

use anyhow::{Context, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Locale {
    En,
    ZhHans,
    ZhHant,
}

impl Locale {
    pub fn from_env() -> Self {
        match env_nonempty("INSIGHTS_LANG")
            .unwrap_or_else(|| "en".to_string())
            .as_str()
        {
            "zh-Hans" => Locale::ZhHans,
            "zh-Hant" => Locale::ZhHant,
            _ => Locale::En,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::ZhHans => "zh-Hans",
            Locale::ZhHant => "zh-Hant",
        }
    }
}

#[derive(Clone, Debug)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub webhook_url: Option<String>,
    pub webhook_port: Option<u16>,
}

#[derive(Clone, Debug)]
pub struct DbConfig {
    pub postgres_url: Option<String>,
    pub sqlite_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub api_base: Option<String>,
    pub model: String,
    pub token_limit: Option<u32>,
    #[allow(dead_code)]
    pub recap_token_limit: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub locale: Locale,
    pub telegram: TelegramConfig,
    pub db: DbConfig,
    pub openai: OpenAiConfig,
    pub log_level: String,
    pub log_file_path: Option<String>,
    pub locales_dir: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let locale = Locale::from_env();

        let telegram = TelegramConfig {
            bot_token: env_nonempty("TELEGRAM_BOT_TOKEN")
                .context("TELEGRAM_BOT_TOKEN is required")?,
            webhook_url: env_nonempty("TELEGRAM_BOT_WEBHOOK_URL"),
            webhook_port: env::var("TELEGRAM_BOT_WEBHOOK_PORT")
                .ok()
                .and_then(|s| s.parse::<u16>().ok()),
        };

        let postgres_url = env_nonempty("DATABASE_URL").or_else(|| env_nonempty("DB_CONNECTION_STR"));

        // Only use SQLite as fallback if:
        // 1. SQLITE_PATH is explicitly set, OR
        // 2. No PostgreSQL URL is configured (use SQLite as default)
        let sqlite_file = env_nonempty("SQLITE_PATH").or_else(|| {
            if postgres_url.is_none() {
                Some("data/dev.db".into())
            } else {
                None
            }
        });

        let db = DbConfig {
            postgres_url,
            sqlite_file,
        };

        let api_base = env_nonempty("OPENAI_API_BASE_URL")
            .or_else(|| env_nonempty("OPENAI_BASE_URL"))
            .or_else(|| env_nonempty("OPENAI_API_HOST"))
            .map(|url| {
                // Normalize URL: remove trailing slash and ensure /v1 suffix
                let url = url.trim_end_matches('/');
                if url.ends_with("/v1") {
                    url.to_string()
                } else {
                    format!("{url}/v1")
                }
            });

        let openai = OpenAiConfig {
            api_key: env_nonempty("OPENAI_API_KEY")
                .context("OPENAI_API_KEY is required")?,
            api_base,
            model: env_nonempty("OPENAI_API_MODEL_NAME").unwrap_or_else(|| "gpt-5".into()),
            token_limit: env::var("OPENAI_API_TOKEN_LIMIT")
                .ok()
                .and_then(|s| s.parse::<u32>().ok()),
            recap_token_limit: env::var("OPENAI_API_CHAT_HISTORIES_RECAP_TOKEN_LIMIT")
                .ok()
                .and_then(|s| s.parse::<u32>().ok()),
        };

        let log_level = env_nonempty("LOG_LEVEL").unwrap_or_else(|| "info".into());
        let log_file_path = env_nonempty("LOG_FILE_PATH");
        let locales_dir = env_nonempty("LOCALES_DIR").unwrap_or_else(|| "./locales".into());

        Ok(Self {
            locale,
            telegram,
            db,
            openai,
            log_level,
            log_file_path,
            locales_dir,
        })
    }
}

fn env_nonempty(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}
