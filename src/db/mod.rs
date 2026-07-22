use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;

pub mod models;
pub mod provider;
pub mod repository;

use crate::config::{Config, DatabaseBackend};
use crate::db::provider::DatabaseProvider;

#[cfg(feature = "sqlite")]
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

#[cfg(feature = "postgres")]
use sqlx::postgres::PgPoolOptions;

/// Database connection pool handle owned by the runtime for lifecycle
/// management. Keeps `DatabaseProvider` and repository traits free of
/// lifecycle methods.
pub enum PoolHandle {
    #[cfg(feature = "sqlite")]
    Sqlite(SqlitePool),
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
}

impl PoolHandle {
    /// Close the connection pool, waiting for all borrowed connections
    /// to be returned. Active transactions will finish before the pool
    /// is fully closed.
    pub async fn close(&self) {
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite(pool) => {
                pool.close().await;
                tracing::info!("sqlite connection pool closed");
            }
            #[cfg(feature = "postgres")]
            Self::Postgres(pool) => {
                pool.close().await;
                tracing::info!("postgres connection pool closed");
            }
        }
    }
}

/// Initialize database connection pool, run migrations, and return the
/// `DatabaseProvider`, detected backend, and a `PoolHandle` for the runtime
/// to manage the pool lifecycle independently of the repositories.
pub async fn init_provider(
    config: &Config,
) -> Result<(Arc<dyn DatabaseProvider>, DatabaseBackend, PoolHandle)> {
    let (url, backend) = config.database.resolved_url()?;

    match backend {
        #[cfg(feature = "sqlite")]
        DatabaseBackend::Sqlite => {
            let path = config.database.sqlite_path();
            if let Some(parent) = std::path::Path::new(&path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!("failed to create database directory: {}", parent.display())
                    })?;
                }
            }

            let max_conn = config.database.max_connections.unwrap_or(16);
            let min_conn = config.database.min_connections.unwrap_or(1);

            let mut opts = SqlitePoolOptions::new()
                .max_connections(max_conn)
                .min_connections(min_conn);

            if let Some(secs) = config.database.connect_timeout_secs {
                opts = opts.acquire_timeout(Duration::from_secs(secs));
            }
            if let Some(secs) = config.database.idle_timeout_secs {
                opts = opts.idle_timeout(Duration::from_secs(secs));
            }
            if let Some(secs) = config.database.max_lifetime_secs {
                opts = opts.max_lifetime(Duration::from_secs(secs));
            }

            let pool = opts
                .connect(&url)
                .await
                .with_context(|| format!("failed to open sqlite database: {url}"))?;

            sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&pool)
                .await
                .context("PRAGMA journal_mode")?;
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(&pool)
                .await
                .context("PRAGMA foreign_keys")?;
            sqlx::query("PRAGMA busy_timeout = 5000")
                .execute(&pool)
                .await
                .context("PRAGMA busy_timeout")?;
            sqlx::query("PRAGMA synchronous = NORMAL")
                .execute(&pool)
                .await
                .context("PRAGMA synchronous")?;

            sqlx::migrate!("src/db/migrations/sqlite")
                .run(&pool)
                .await
                .context("failed to run sqlite migrations")?;

            tracing::info!(backend = "sqlite", url = %url, "sqlite database initialized");
            let pool_handle = PoolHandle::Sqlite(pool.clone());
            let provider = Arc::new(provider::SqliteProvider::new(pool));
            Ok((provider, DatabaseBackend::Sqlite, pool_handle))
        }

        #[cfg(feature = "postgres")]
        DatabaseBackend::Postgres => {
            let max_conn = config.database.max_connections.unwrap_or(16);
            let min_conn = config.database.min_connections.unwrap_or(1);

            let mut opts = PgPoolOptions::new()
                .max_connections(max_conn)
                .min_connections(min_conn);

            if let Some(secs) = config.database.connect_timeout_secs {
                opts = opts.acquire_timeout(Duration::from_secs(secs));
            }
            if let Some(secs) = config.database.idle_timeout_secs {
                opts = opts.idle_timeout(Duration::from_secs(secs));
            }
            if let Some(secs) = config.database.max_lifetime_secs {
                opts = opts.max_lifetime(Duration::from_secs(secs));
            }

            // Retry connection policy (5 attempts with exponential backoff)
            let mut attempts = 0;
            let mut wait_secs = 1u64;
            let pool = loop {
                match opts.clone().connect(&url).await {
                    Ok(p) => break p,
                    Err(err) => {
                        attempts += 1;
                        if attempts >= 5 {
                            anyhow::bail!(
                                "failed to connect to postgres database after {attempts} attempts: {err}"
                            );
                        }
                        tracing::warn!(
                            attempts,
                            wait_secs,
                            "postgres connection failed, retrying..."
                        );
                        tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                        wait_secs = std::cmp::min(wait_secs * 2, 30);
                    }
                }
            };

            sqlx::migrate!("src/db/migrations/postgres")
                .run(&pool)
                .await
                .context("failed to run postgres migrations")?;

            tracing::info!(backend = "postgres", url = %url, "postgres database initialized");
            let pool_handle = PoolHandle::Postgres(pool.clone());
            let provider = Arc::new(provider::PostgresProvider::new(pool));
            Ok((provider, DatabaseBackend::Postgres, pool_handle))
        }

        #[allow(unreachable_patterns)]
        _ => anyhow::bail!("database backend '{backend}' feature is not enabled in this build"),
    }
}

/// Helper function to create an SQLite pool for legacy CLI commands or tests.
#[cfg(feature = "sqlite")]
pub async fn create_pool(path: &str) -> Result<SqlitePool> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create database directory: {}", parent.display())
            })?;
        }
    }
    let url = if path.starts_with("sqlite://") {
        path.to_string()
    } else {
        format!("sqlite://{}?mode=rwc", path)
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(16)
        .min_connections(1)
        .connect(&url)
        .await
        .with_context(|| format!("failed to open sqlite database: {path}"))?;

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await
        .context("PRAGMA journal_mode")?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .context("PRAGMA foreign_keys")?;

    Ok(pool)
}

#[cfg(feature = "sqlite")]
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("src/db/migrations/sqlite")
        .run(pool)
        .await
        .context("failed to run sqlite migrations")?;
    Ok(())
}
