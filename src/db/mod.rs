use anyhow::{Context, Result};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

/// Create and configure the SQLite connection pool.
///
/// Enables WAL mode, foreign keys, and a busy timeout so concurrent writers
/// do not immediately error — they back off and retry for up to 5 seconds.
pub async fn create_pool(path: &str) -> Result<SqlitePool> {
    // Ensure the parent directory exists
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

    // Apply foundational PRAGMAs on every connection
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

    sqlx::query("PRAGMA cache_size = -32768") // 32 MiB page cache
        .execute(&pool)
        .await
        .context("PRAGMA cache_size")?;

    tracing::info!(path = path, "database pool opened");
    Ok(pool)
}

/// Run all pending SQLx migrations embedded in `src/db/migrations/`.
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("src/db/migrations")
        .run(pool)
        .await
        .context("failed to run database migrations")?;
    tracing::info!("database migrations applied");
    Ok(())
}

pub mod models;
pub mod repository;
