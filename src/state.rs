use crate::db::provider::DatabaseProvider;
use std::sync::Arc;

use crate::{config::Config, security::RateLimiter};

/// Shared application state injected into every Axum handler via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub provider: Arc<dyn DatabaseProvider>,
    pub config: Arc<Config>,
    pub rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(provider: Arc<dyn DatabaseProvider>, config: Config) -> Self {
        Self {
            provider,
            config: Arc::new(config),
            rate_limiter: RateLimiter::new(),
        }
    }
}
