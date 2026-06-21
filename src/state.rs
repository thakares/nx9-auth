use std::sync::Arc;

use sqlx::SqlitePool;

use crate::{config::Config, security::RateLimiter};

/// Shared application state injected into every Axum handler via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(pool: SqlitePool, config: Config) -> Self {
        Self {
            pool,
            config: Arc::new(config),
            rate_limiter: RateLimiter::new(),
        }
    }
}
