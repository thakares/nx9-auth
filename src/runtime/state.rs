//! Lock-free atomic runtime state machine.
//!
//! Tracks the application through granular lifecycle phases using `AtomicU8`
//! with `compare_exchange` transitions. This avoids mutex contention and
//! provides deterministic, race-free state management across async tasks.

use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};

/// Granular runtime lifecycle states for enterprise production observability.
///
/// Each transition is deterministic and logged. Only forward transitions
/// are permitted during normal operation; the state machine never moves
/// backward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum RuntimeState {
    /// Runtime is being configured and dependencies are being assembled.
    Initializing = 0,
    /// Subsystems are starting (database, HTTP listener, workers).
    Starting = 1,
    /// Application is fully operational and serving requests.
    Running = 2,
    /// Shutdown signal received; HTTP listener stopped, draining active requests.
    Draining = 3,
    /// Active requests drained; cancelling and awaiting background workers.
    StoppingWorkers = 4,
    /// Workers stopped; executing registered shutdown hooks.
    ExecutingHooks = 5,
    /// Hooks executed; closing database connection pools and flushing logs.
    ClosingResources = 6,
    /// All resources released; process is ready to exit.
    Stopped = 7,
}

impl RuntimeState {
    /// Convert a raw `u8` value back into a `RuntimeState`.
    fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Initializing),
            1 => Some(Self::Starting),
            2 => Some(Self::Running),
            3 => Some(Self::Draining),
            4 => Some(Self::StoppingWorkers),
            5 => Some(Self::ExecutingHooks),
            6 => Some(Self::ClosingResources),
            7 => Some(Self::Stopped),
            _ => None,
        }
    }

    /// Returns `true` if the runtime is in any shutdown phase.
    pub fn is_shutting_down(self) -> bool {
        (self as u8) >= (Self::Draining as u8)
    }
}

impl fmt::Display for RuntimeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initializing => write!(f, "Initializing"),
            Self::Starting => write!(f, "Starting"),
            Self::Running => write!(f, "Running"),
            Self::Draining => write!(f, "Draining"),
            Self::StoppingWorkers => write!(f, "StoppingWorkers"),
            Self::ExecutingHooks => write!(f, "ExecutingHooks"),
            Self::ClosingResources => write!(f, "ClosingResources"),
            Self::Stopped => write!(f, "Stopped"),
        }
    }
}

/// Lock-free atomic runtime state container.
///
/// Uses `AtomicU8` with `compare_exchange` to ensure deterministic,
/// race-free state transitions without mutex contention.
pub struct AtomicRuntimeState {
    state: AtomicU8,
}

impl AtomicRuntimeState {
    /// Create a new state machine in the `Initializing` state.
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(RuntimeState::Initializing as u8),
        }
    }

    /// Read the current state (acquire ordering for visibility).
    pub fn load(&self) -> RuntimeState {
        RuntimeState::from_u8(self.state.load(Ordering::Acquire)).unwrap_or(RuntimeState::Stopped)
    }

    /// Attempt an atomic state transition from `expected` to `new`.
    ///
    /// Returns `Ok(new)` if the transition succeeded, or `Err(actual)` if the
    /// current state did not match `expected`.
    pub fn transition(
        &self,
        expected: RuntimeState,
        new: RuntimeState,
    ) -> Result<RuntimeState, RuntimeState> {
        match self.state.compare_exchange(
            expected as u8,
            new as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                tracing::info!(from = %expected, to = %new, "runtime state transition");
                Ok(new)
            }
            Err(actual) => {
                let actual_state = RuntimeState::from_u8(actual).unwrap_or(RuntimeState::Stopped);
                tracing::debug!(
                    expected = %expected,
                    actual = %actual_state,
                    target = %new,
                    "state transition skipped (unexpected current state)"
                );
                Err(actual_state)
            }
        }
    }

    /// Unconditionally advance the state. Used during forced shutdown when
    /// intermediate states may have been skipped.
    pub fn force_set(&self, new: RuntimeState) {
        let prev = self.state.swap(new as u8, Ordering::AcqRel);
        let prev_state = RuntimeState::from_u8(prev).unwrap_or(RuntimeState::Stopped);
        if prev_state != new {
            tracing::info!(from = %prev_state, to = %new, "runtime state forced");
        }
    }

    /// Attempt to transition from `Running` to `Draining`.
    ///
    /// Returns `true` if this call initiated shutdown (first caller wins).
    /// Returns `false` if shutdown was already in progress or the runtime
    /// was not yet `Running`.
    pub fn initiate_shutdown(&self) -> bool {
        self.transition(RuntimeState::Running, RuntimeState::Draining)
            .is_ok()
    }
}

impl Default for AtomicRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transitions() {
        let state = AtomicRuntimeState::new();
        assert_eq!(state.load(), RuntimeState::Initializing);

        assert!(
            state
                .transition(RuntimeState::Initializing, RuntimeState::Starting)
                .is_ok()
        );
        assert_eq!(state.load(), RuntimeState::Starting);

        assert!(
            state
                .transition(RuntimeState::Starting, RuntimeState::Running)
                .is_ok()
        );
        assert_eq!(state.load(), RuntimeState::Running);

        // Duplicate shutdown protection
        assert!(state.initiate_shutdown());
        assert!(!state.initiate_shutdown()); // second call fails
        assert_eq!(state.load(), RuntimeState::Draining);
    }

    #[test]
    fn test_force_set() {
        let state = AtomicRuntimeState::new();
        state.force_set(RuntimeState::ClosingResources);
        assert_eq!(state.load(), RuntimeState::ClosingResources);
    }

    #[test]
    fn test_is_shutting_down() {
        assert!(!RuntimeState::Initializing.is_shutting_down());
        assert!(!RuntimeState::Starting.is_shutting_down());
        assert!(!RuntimeState::Running.is_shutting_down());
        assert!(RuntimeState::Draining.is_shutting_down());
        assert!(RuntimeState::StoppingWorkers.is_shutting_down());
        assert!(RuntimeState::ExecutingHooks.is_shutting_down());
        assert!(RuntimeState::ClosingResources.is_shutting_down());
        assert!(RuntimeState::Stopped.is_shutting_down());
    }
}
