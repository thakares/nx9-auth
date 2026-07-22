//! Common lifecycle contract for runtime components.

use anyhow::Result;

/// Lifecycle contract for the application runtime.
#[async_trait::async_trait]
pub trait Lifecycle {
    /// Initialize subsystems.
    async fn initialize(&mut self) -> Result<()>;

    /// Start serving.
    async fn start(&mut self) -> Result<()>;

    /// Perform graceful shutdown.
    async fn shutdown(&mut self) -> Result<()>;
}
