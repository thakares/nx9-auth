use anyhow::{Context, Result};
#[cfg(feature = "sqlite")]
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

#[cfg(feature = "sqlite")]
pub async fn create_pool(path: &str) -> Result<SqlitePool> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create database directory: {}", parent.display())
            })?;
        }
    }

    let url = format!("sqlite://{}?mode=rwc", path);
    let pool = SqlitePoolOptions::new()
        .max_connections(16)
        .min_connections(1)
        .connect(&url)
        .await
        .with_context(|| format!("failed to open database: {path}"))?;

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
    sqlx::query("PRAGMA cache_size = -32768")
        .execute(&pool)
        .await
        .context("PRAGMA cache_size")?;

    tracing::info!(path = path, "database pool opened");
    Ok(pool)
}

#[cfg(feature = "sqlite")]
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("src/db/migrations/sqlite")
        .run(pool)
        .await
        .context("failed to run database migrations")?;
    tracing::info!("database migrations applied");
    Ok(())
}

#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub async fn create_pool(url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(16)
        .min_connections(1)
        .connect(url)
        .await
        .with_context(|| format!("failed to open database: {url}"))?;

    tracing::info!(url = url, "postgres pool opened");
    Ok(pool)
}

#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("src/db/migrations/postgres")
        .run(pool)
        .await
        .context("failed to run postgres migrations")?;
    tracing::info!("postgres migrations applied");
    Ok(())
}

pub mod models;
pub mod provider;
pub mod repository;
