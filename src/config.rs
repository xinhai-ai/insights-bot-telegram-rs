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
        match env::var("INSIGHTS_LANG")
            .unwrap_or_else(|_| "en".to_string())
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
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub locale: Locale,
    pub telegram: TelegramConfig,
    pub db: DbConfig,
    pub openai: OpenAiConfig,
    pub log_level: String,
    pub locales_dir: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let locale = Locale::from_env();

        let telegram = TelegramConfig {
            bot_token: env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN is required")?,
            webhook_url: env::var("TELEGRAM_BOT_WEBHOOK_URL").ok(),
            webhook_port: env::var("TELEGRAM_BOT_WEBHOOK_PORT")
                .ok()
                .and_then(|s| s.parse::<u16>().ok()),
        };

        let postgres_url = env::var("DATABASE_URL")
            .ok()
            .or_else(|| env::var("DB_CONNECTION_STR").ok());
        let sqlite_file = env::var("SQLITE_PATH")
            .ok()
            .or_else(|| Some("data/dev.db".into()));

        let db = DbConfig {
            postgres_url,
            sqlite_file,
        };

        let openai = OpenAiConfig {
            api_key: env::var("OPENAI_API_SECRET")
                .or_else(|_| env::var("OPENAI_API_KEY"))
                .context("OPENAI_API_SECRET (or OPENAI_API_KEY) is required")?,
            api_base: env::var("OPENAI_API_HOST").ok(),
            model: env::var("OPENAI_API_MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".into()),
            token_limit: env::var("OPENAI_API_TOKEN_LIMIT")
                .ok()
                .and_then(|s| s.parse::<u32>().ok()),
        };

        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into());
        let locales_dir = env::var("LOCALES_DIR").unwrap_or_else(|_| "./locales".into());

        Ok(Self {
            locale,
            telegram,
            db,
            openai,
            log_level,
            locales_dir,
        })
    }
}
