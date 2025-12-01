mod bot;
mod config;
mod db;
mod http;
mod i18n;
mod services;
mod telemetry;

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = config::AppConfig::from_env()?;
    telemetry::init_tracing(&config)?;

    let database = db::Database::connect_from_env(&config.db).await?;
    let i18n = i18n::I18n::load_from_dir(&config.locales_dir)?;
    let openai = services::openai::OpenAiClient::new(&config.openai)?;
    let limiter = services::rate_limit::CommandRateLimiter::new(1);
    let ctx = bot::context::AppContext::new(config, database.clone(), i18n, openai, limiter);

    info!(
        "bootstrap completed (backend: {:?}, locale: {})",
        database.backend,
        ctx.config.locale.code()
    );

    // Start background services.
    services::autorecap::spawn_autorecap(ctx.clone()).await;

    // Health endpoint (optional port 3000).
    let health_addr = "0.0.0.0:3000".parse().unwrap();
    http::health::serve(database, health_addr);

    // Start bot dispatcher.
    bot::run(ctx).await?;
    Ok(())
}
