//! Runtime-wide shutdown coordination using `CancellationToken`.

use tokio_util::sync::CancellationToken;

/// Dual-token shutdown coordinator that supports graceful termination
/// (1st signal) and live forced escalation (2nd signal).
#[derive(Clone)]
pub struct ShutdownCoordinator {
    graceful: CancellationToken,
    forced: CancellationToken,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        Self {
            graceful: CancellationToken::new(),
            forced: CancellationToken::new(),
        }
    }

    /// Access the primary graceful cancellation token.
    pub fn token(&self) -> &CancellationToken {
        &self.graceful
    }

    /// Access the forced cancellation token.
    pub fn forced_token(&self) -> &CancellationToken {
        &self.forced
    }

    /// Create a child token linked to graceful cancellation.
    pub fn child_token(&self) -> CancellationToken {
        self.graceful.child_token()
    }

    /// Trigger graceful shutdown.
    pub fn cancel(&self) {
        self.graceful.cancel();
    }

    /// Trigger graceful shutdown explicitly.
    pub fn cancel_graceful(&self) {
        self.graceful.cancel();
    }

    /// Trigger forced shutdown escalation live.
    pub fn cancel_forced(&self) {
        self.graceful.cancel();
        self.forced.cancel();
    }

    /// Check if graceful shutdown has been initiated.
    pub fn is_cancelled(&self) -> bool {
        self.graceful.is_cancelled()
    }

    /// Check if forced escalation has been triggered.
    pub fn is_forced(&self) -> bool {
        self.forced.is_cancelled()
    }

    /// Await graceful cancellation.
    pub async fn cancelled(&self) {
        self.graceful.cancelled().await;
    }

    /// Await graceful cancellation explicitly.
    pub async fn graceful_cancelled(&self) {
        self.graceful.cancelled().await;
    }

    /// Await forced escalation live.
    pub async fn forced_cancelled(&self) {
        self.forced.cancelled().await;
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
