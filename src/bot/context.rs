use std::sync::Arc;

use crate::{
    config::AppConfig,
    db::Database,
    i18n::I18n,
    services::{openai::OpenAiClient, rate_limit::CommandRateLimiter},
};

#[derive(Clone)]
pub struct AppContext {
    pub config: AppConfig,
    pub db: Database,
    pub i18n: I18n,
    pub openai: OpenAiClient,
    pub limiter: CommandRateLimiter,
}

impl AppContext {
    pub fn new(
        config: AppConfig,
        db: Database,
        i18n: I18n,
        openai: OpenAiClient,
        limiter: CommandRateLimiter,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            db,
            i18n,
            openai,
            limiter,
        })
    }
}
