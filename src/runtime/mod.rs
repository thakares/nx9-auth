//! Unified Enterprise Runtime Lifecycle for `nx9-auth`.
//!
//! Provides application assembly (`ApplicationBuilder`), lifecycle management
//! (`Application`, `Lifecycle`), atomic state machine (`RuntimeState`),
//! signal coordination (`SignalManager`), prioritized shutdown hooks
//! (`ShutdownHook`), worker management (`WorkerManager`), and operational
//! metrics (`RuntimeMetrics`).

pub mod application;
pub mod builder;
pub mod cancellation;
pub mod hooks;
pub mod lifecycle;
pub mod metrics;
pub mod signals;
pub mod state;
pub mod workers;

pub use application::Application;
pub use builder::ApplicationBuilder;
pub use cancellation::ShutdownCoordinator;
pub use hooks::{HookRegistry, ShutdownHook, ShutdownPriority};
pub use lifecycle::Lifecycle;
pub use metrics::RuntimeMetrics;
pub use signals::SignalManager;
pub use state::{AtomicRuntimeState, RuntimeState};
pub use workers::{TaskGroup, WorkerManager};
