use anyhow::{Context, Result};
use sqlx::{AnyPool, any::AnyPoolOptions};
use tracing::{info, warn};

use crate::config::DbConfig;

pub mod chat_history;
pub mod logs;
pub mod models;
pub mod recap_config;

#[derive(Debug, Clone, Copy)]
pub enum DbBackend {
    Postgres,
    Sqlite,
}

#[derive(Clone)]
pub struct Database {
    pub pool: AnyPool,
    pub backend: DbBackend,
}

impl Database {
    pub async fn connect_from_env(cfg: &DbConfig) -> Result<Self> {
        sqlx::any::install_default_drivers();

        if let Some(url) = cfg.postgres_url.as_ref() {
            match AnyPoolOptions::new().connect(url).await {
                Ok(pool) => {
                    info!("connected to Postgres");
                    return Ok(Self {
                        pool,
                        backend: DbBackend::Postgres,
                    });
                }
                Err(err) => {
                    warn!("Postgres connection failed: {err}; falling back to SQLite");
                }
            }
        }

        let sqlite_path = cfg
            .sqlite_file
            .clone()
            .unwrap_or_else(|| "data/dev.db".to_string());
        let sqlite_url = if sqlite_path.starts_with("sqlite://") {
            sqlite_path
        } else {
            format!("sqlite://{sqlite_path}")
        };

        let pool = AnyPoolOptions::new()
            .connect(&sqlite_url)
            .await
            .with_context(|| format!("SQLite connect failed ({sqlite_url})"))?;

        info!("connected to SQLite at {sqlite_url}");
        Ok(Self {
            pool,
            backend: DbBackend::Sqlite,
        })
    }
}
