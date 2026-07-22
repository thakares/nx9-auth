//! Unix signal handling for graceful and forced shutdown.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Clone)]
pub struct SignalManager {
    signal_count: Arc<AtomicUsize>,
    force_shutdown: Arc<AtomicBool>,
}

impl SignalManager {
    pub fn new() -> Self {
        Self {
            signal_count: Arc::new(AtomicUsize::new(0)),
            force_shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_force_shutdown(&self) -> bool {
        self.force_shutdown.load(Ordering::Acquire)
    }

    pub fn signal_count(&self) -> usize {
        self.signal_count.load(Ordering::Acquire)
    }

    pub fn record_signal(&self) -> usize {
        let count = self.signal_count.fetch_add(1, Ordering::AcqRel) + 1;
        if count >= 2 {
            self.force_shutdown.store(true, Ordering::Release);
        }
        count
    }
}

impl Default for SignalManager {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn wait_for_shutdown_signal() -> &'static str {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install SIGINT handler");
        "SIGINT"
    };

    #[cfg(unix)]
    let sigterm = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
        "SIGTERM"
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<&str>();

    tokio::select! {
        name = ctrl_c => name,
        name = sigterm => name,
    }
}
