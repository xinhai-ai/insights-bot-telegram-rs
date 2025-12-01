use anyhow::Result;
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt};

use crate::config::AppConfig;

pub fn init_tracing(config: &AppConfig) -> Result<()> {
    let filter = EnvFilter::try_new(&config.log_level).or_else(|_| EnvFilter::try_new("info"))?;

    let subscriber = Registry::default()
        .with(filter)
        .with(fmt::layer().with_target(true).with_line_number(true));

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
