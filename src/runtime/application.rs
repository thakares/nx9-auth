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
    pub local_addr: Option<std::net::SocketAddr>,
    pub bound_port: Arc<std::sync::atomic::AtomicU16>,
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

    /// Force advance runtime state (monotonic, forward-only).
    pub fn set_state(&self, state: RuntimeState) {
        self.state.force_advance(state);
    }

    /// Perform graceful shutdown flow explicitly.
    pub async fn perform_shutdown(&mut self) -> Result<()> {
        let current_state = self.state.load();

        if current_state == RuntimeState::Running {
            let _ = self.state.initiate_shutdown();
        } else if !current_state.is_shutting_down() {
            self.state.force_advance(RuntimeState::Draining);
        }

        if self.state.load() == RuntimeState::Draining {
            tracing::info!("draining active connections");
            let _ = self
                .state
                .transition(RuntimeState::Draining, RuntimeState::StoppingWorkers);
        }

        if self.state.load() == RuntimeState::StoppingWorkers {
            tracing::info!("stopping background workers");
            self.workers
                .shutdown_all_with_coordinator(Duration::from_secs(10), Some(&self.shutdown))
                .await;
            let _ = self
                .state
                .transition(RuntimeState::StoppingWorkers, RuntimeState::ExecutingHooks);
        }

        if self.state.load() == RuntimeState::ExecutingHooks {
            tracing::info!("executing shutdown hooks");
            self.hooks.execute_all().await;
            let _ = self
                .state
                .transition(RuntimeState::ExecutingHooks, RuntimeState::ClosingResources);
        }

        if self.state.load() == RuntimeState::ClosingResources {
            tracing::info!("closing database connection pool and resources");
            if let Some(pool) = self.pool_handle.take() {
                pool.close().await;
            }
            let _ = self
                .state
                .transition(RuntimeState::ClosingResources, RuntimeState::Stopped);
        }

        tracing::info!("application stopped cleanly");
        Ok(())
    }
}

#[async_trait::async_trait]
impl Lifecycle for Application {
    async fn initialize(&mut self) -> Result<()> {
        let _ = self
            .state
            .transition(RuntimeState::Initializing, RuntimeState::Starting);

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

        let config = self.config.as_ref().cloned().unwrap_or_default();
        let addr_str = format!("{}:{}", config.server.host, config.server.port);
        let listener = tokio::net::TcpListener::bind(&addr_str)
            .await
            .with_context(|| format!("failed to bind TCP listener to {addr_str}"))?;

        let local_addr = listener.local_addr()?;
        self.local_addr = Some(local_addr);
        self.bound_port
            .store(local_addr.port(), std::sync::atomic::Ordering::Release);
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
        let state = self.state.clone();

        let signal_task = tokio::spawn(signals::listen_for_signals(
            signal_mgr,
            shutdown_coord.clone(),
            self.state.clone(),
        ));

        let graceful_token = shutdown_coord.token().clone();
        let forced_token = shutdown_coord.forced_token().clone();

        let state_for_shutdown = state.clone();
        let server_fut = axum::serve(listener, router).with_graceful_shutdown(async move {
            graceful_token.cancelled().await;
            tracing::info!("graceful shutdown triggered; initiating HTTP connection draining");
            let _ = state_for_shutdown.initiate_shutdown();
        });

        let mut server_task = tokio::spawn(async move { server_fut.await });

        tokio::select! {
            res = &mut server_task => {
                match res {
                    Ok(Ok(())) => tracing::info!("HTTP server stopped gracefully"),
                    Ok(Err(err)) => tracing::error!(error = %err, "HTTP server error"),
                    Err(join_err) => tracing::debug!(error = %join_err, "HTTP server task finished"),
                }
            }
            _ = forced_token.cancelled() => {
                tracing::warn!("live forced shutdown escalation received during HTTP drain; aborting server task immediately");
                server_task.abort();
                let _ = server_task.await;
            }
        }

        signal_task.abort();
        self.perform_shutdown().await
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.perform_shutdown().await
    }
}
