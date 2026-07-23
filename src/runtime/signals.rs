//! Unix signal handling for graceful and forced shutdown.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::{AtomicRuntimeState, ShutdownCoordinator};

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

    /// Record a signal and trigger live escalation on the coordinator.
    pub fn handle_signal(&self, coordinator: &ShutdownCoordinator) -> usize {
        let count = self.record_signal();
        if count == 1 {
            coordinator.cancel_graceful();
        } else if count >= 2 {
            coordinator.cancel_forced();
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

/// Continuous signal monitor that remains active during graceful shutdown
/// to observe and trigger forced escalation live.
pub async fn listen_for_signals(
    signal_mgr: SignalManager,
    coordinator: ShutdownCoordinator,
    state: AtomicRuntimeState,
) {
    loop {
        let sig = wait_for_shutdown_signal().await;
        let count = signal_mgr.handle_signal(&coordinator);
        tracing::info!(signal = sig, count, "received OS signal");
        if count == 1 {
            let _ = state.initiate_shutdown();
        } else {
            // 2nd signal received: forced escalation
            tracing::warn!("second signal received; escalating to forced shutdown");
            break;
        }
    }
}
