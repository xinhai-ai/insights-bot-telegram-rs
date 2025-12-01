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
                    let db = Self {
                        pool,
                        backend: DbBackend::Postgres,
                    };
                    db.run_migrations().await?;
                    return Ok(db);
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
            // Ensure create_if_missing is enabled.
            if sqlite_path.contains('?') {
                sqlite_path
            } else {
                format!("{sqlite_path}?mode=rwc")
            }
        } else {
            format!("sqlite://{sqlite_path}?mode=rwc")
        };

        let pool = AnyPoolOptions::new()
            .connect(&sqlite_url)
            .await
            .with_context(|| format!("SQLite connect failed ({sqlite_url})"))?;

        info!("connected to SQLite at {sqlite_url}");
        let db = Self {
            pool,
            backend: DbBackend::Sqlite,
        };
        db.run_migrations().await?;
        Ok(db)
    }

    /// Run database migrations based on the backend type.
    async fn run_migrations(&self) -> Result<()> {
        let migration_sql = match self.backend {
            DbBackend::Postgres => include_str!("../../migrations/postgres/0001_init.sql"),
            DbBackend::Sqlite => include_str!("../../migrations/sqlite/0001_init.sql"),
        };

        // Execute each statement separately (SQLite doesn't support multiple statements in one query)
        for statement in migration_sql.split(';') {
            // Strip leading comment lines (lines starting with --)
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
            sqlx::query(stmt).execute(&self.pool).await?;
        }

        info!("database migrations completed");
        Ok(())
    }
}
