//! Runtime-wide shutdown coordination using `CancellationToken`.

use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct ShutdownCoordinator {
    root: CancellationToken,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        Self {
            root: CancellationToken::new(),
        }
    }

    pub fn token(&self) -> &CancellationToken {
        &self.root
    }

    pub fn child_token(&self) -> CancellationToken {
        self.root.child_token()
    }

    pub fn cancel(&self) {
        self.root.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.root.is_cancelled()
    }

    pub async fn cancelled(&self) {
        self.root.cancelled().await;
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
