//! Runtime application container.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::db::PoolHandle;
use crate::db::provider::DatabaseProvider;

use super::{
    AtomicRuntimeState, HookRegistry, Lifecycle, RuntimeMetrics, RuntimeState, ShutdownCoordinator,
    SignalManager, WorkerManager, signals,
};

/// Unified runtime application container that manages state transitions,
/// database connections, router setup, HTTP server execution, background workers,
/// metrics, and graceful shutdown hooks.
#[derive(Default)]
pub struct Application {
    pub config: Option<Config>,
    pub provider: Option<Arc<dyn DatabaseProvider>>,
    pub pool_handle: Option<PoolHandle>,
    pub router: Option<axum::Router>,
    pub state: AtomicRuntimeState,
    pub hooks: HookRegistry,
    pub workers: WorkerManager,
    pub signals: SignalManager,
    pub shutdown: ShutdownCoordinator,
    pub metrics: RuntimeMetrics,
}

impl Application {
    /// Create a new application runtime.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for a runtime application initialized from config.
    pub fn builder(config: Config) -> super::ApplicationBuilder {
        super::ApplicationBuilder::new().with_config(config)
    }

    /// Read the current runtime state.
    pub fn state(&self) -> RuntimeState {
        self.state.load()
    }

    /// Access the shutdown hook registry.
    pub fn hooks(&self) -> &HookRegistry {
        &self.hooks
    }

    /// Mutably access the shutdown hook registry.
    pub fn hooks_mut(&mut self) -> &mut HookRegistry {
        &mut self.hooks
    }

    /// Access the worker manager.
    pub fn workers(&self) -> &WorkerManager {
        &self.workers
    }

    /// Mutably access the worker manager.
    pub fn workers_mut(&mut self) -> &mut WorkerManager {
        &mut self.workers
    }

    /// Access the signal manager.
    pub fn signals(&self) -> &SignalManager {
        &self.signals
    }

    /// Access the shutdown coordinator.
    pub fn shutdown_coordinator(&self) -> &ShutdownCoordinator {
        &self.shutdown
    }

    /// Access runtime metrics.
    pub fn metrics(&self) -> &RuntimeMetrics {
        &self.metrics
    }

    /// Force a runtime state update.
    pub fn set_state(&self, state: RuntimeState) {
        self.state.force_set(state);
    }

    /// Perform graceful shutdown flow explicitly.
    pub async fn perform_shutdown(&mut self) -> Result<()> {
        if !self.state.initiate_shutdown() {
            if self.state.load().is_shutting_down() {
                return Ok(());
            }
            self.state.force_set(RuntimeState::Draining);
        }

        println!("Draining");
        tracing::info!("draining active connections");

        let _ = self
            .state
            .transition(RuntimeState::Draining, RuntimeState::StoppingWorkers);
        println!("StoppingWorkers");
        tracing::info!("stopping background workers");
        self.workers.shutdown_all(Duration::from_secs(10)).await;

        let _ = self
            .state
            .transition(RuntimeState::StoppingWorkers, RuntimeState::ExecutingHooks);
        println!("ExecutingHooks");
        tracing::info!("executing shutdown hooks");
        self.hooks.execute_all().await;

        let _ = self
            .state
            .transition(RuntimeState::ExecutingHooks, RuntimeState::ClosingResources);
        println!("ClosingResources");
        tracing::info!("closing database connection pool and resources");
        if let Some(pool) = self.pool_handle.take() {
            pool.close().await;
        }

        let _ = self
            .state
            .transition(RuntimeState::ClosingResources, RuntimeState::Stopped);
        println!("Stopped");
        tracing::info!("application stopped cleanly");

        Ok(())
    }
}

#[async_trait::async_trait]
impl Lifecycle for Application {
    async fn initialize(&mut self) -> Result<()> {
        println!("Initializing");
        let _ = self
            .state
            .transition(RuntimeState::Initializing, RuntimeState::Starting);

        println!("Starting");
        let config = match &self.config {
            Some(cfg) => cfg.clone(),
            None => {
                let mut cfg = Config::default();
                cfg.resolve_paths();
                self.config = Some(cfg.clone());
                cfg
            }
        };

        if self.provider.is_none() {
            let (provider, _backend, pool_handle) = crate::db::init_provider(&config).await?;
            self.provider = Some(provider);
            self.pool_handle = Some(pool_handle);
        }

        if self.router.is_none() {
            if let Some(provider) = &self.provider {
                let app_state = crate::state::AppState::new(provider.clone(), config);
                let router = crate::api::router::build(app_state);
                self.router = Some(router);
            }
        }

        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        if self.state.load() == RuntimeState::Initializing {
            self.initialize().await?;
        }

        if self.state.load() == RuntimeState::Starting {
            let _ = self
                .state
                .transition(RuntimeState::Starting, RuntimeState::Running);
        }

        println!("Running");

        let config = self.config.as_ref().cloned().unwrap_or_default();
        let addr_str = format!("{}:{}", config.server.host, config.server.port);
        let listener = tokio::net::TcpListener::bind(&addr_str)
            .await
            .with_context(|| format!("failed to bind TCP listener to {addr_str}"))?;

        let local_addr = listener.local_addr()?;
        println!("Listening on {}", local_addr);
        tracing::info!(address = %local_addr, "Listening on {}", local_addr);

        let router = match self.router.take() {
            Some(r) => r,
            None => {
                let provider = self
                    .provider
                    .clone()
                    .context("database provider not initialized")?;
                let app_state = crate::state::AppState::new(provider, config);
                crate::api::router::build(app_state)
            }
        };

        let signal_mgr = self.signals.clone();
        let shutdown_coord = self.shutdown.clone();

        let server = axum::serve(listener, router).with_graceful_shutdown(async move {
            tokio::select! {
                sig = signals::wait_for_shutdown_signal() => {
                    tracing::info!(signal = sig, "received shutdown signal");
                    signal_mgr.record_signal();
                    shutdown_coord.cancel();
                }
                _ = shutdown_coord.cancelled() => {
                    tracing::info!("shutdown coordinator cancelled");
                }
            }
        });

        if let Err(err) = server.await {
            tracing::error!(error = %err, "HTTP server error");
        }

        self.perform_shutdown().await
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.perform_shutdown().await
    }
}
