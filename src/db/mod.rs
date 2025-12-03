use anyhow::{Context, Result};
use sqlx::{AnyPool, any::AnyPoolOptions};
use tracing::{debug, info, warn};
use url::Url;

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
            match Self::connect_postgres(url).await {
                Ok(db) => return Ok(db),
                Err(err) => {
                    if cfg.sqlite_file.is_some() {
                        warn!("Postgres connection failed: {err}; falling back to SQLite");
                    } else {
                        // No SQLite fallback configured, propagate error
                        return Err(err);
                    }
                }
            }
        }

        // Connect to SQLite if configured
        let Some(sqlite_path) = cfg.sqlite_file.clone() else {
            anyhow::bail!(
                "no database configured: set DATABASE_URL for PostgreSQL or SQLITE_PATH for SQLite"
            );
        };

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

    /// Connect to PostgreSQL, creating the database if it doesn't exist.
    async fn connect_postgres(url: &str) -> Result<Self> {
        // First, try direct connection
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
                let err_str = err.to_string();
                // Check if error is "database does not exist"
                if err_str.contains("does not exist") {
                    debug!("database does not exist, attempting to create it");
                    Self::create_postgres_database(url).await?;
                    // Retry connection after creating database
                    let pool = AnyPoolOptions::new()
                        .connect(url)
                        .await
                        .with_context(|| "failed to connect after creating database")?;
                    info!("connected to Postgres (database was auto-created)");
                    let db = Self {
                        pool,
                        backend: DbBackend::Postgres,
                    };
                    db.run_migrations().await?;
                    return Ok(db);
                }
                return Err(err.into());
            }
        }
    }

    /// Create a PostgreSQL database by connecting to the default 'postgres' database.
    async fn create_postgres_database(url: &str) -> Result<()> {
        let mut parsed = Url::parse(url).with_context(|| "invalid DATABASE_URL")?;

        // Extract database name from path (e.g., "/mydb" -> "mydb")
        let db_name = parsed
            .path()
            .trim_start_matches('/')
            .to_string();

        if db_name.is_empty() {
            anyhow::bail!("DATABASE_URL must specify a database name");
        }

        // Change path to connect to default 'postgres' database
        parsed.set_path("/postgres");
        let admin_url = parsed.as_str();

        debug!("connecting to 'postgres' database to create '{db_name}'");
        let admin_pool = AnyPoolOptions::new()
            .connect(admin_url)
            .await
            .with_context(|| "failed to connect to 'postgres' database for db creation")?;

        // Use dynamic SQL to create database (identifiers can't be parameterized)
        // Validate db_name to prevent SQL injection
        if !db_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            anyhow::bail!("invalid database name: {db_name}");
        }

        let create_sql = format!("CREATE DATABASE \"{db_name}\"");
        sqlx::query(&create_sql)
            .execute(&admin_pool)
            .await
            .with_context(|| format!("failed to create database '{db_name}'"))?;

        info!("created PostgreSQL database '{db_name}'");
        admin_pool.close().await;
        Ok(())
    }
}
