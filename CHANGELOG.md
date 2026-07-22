# Changelog

All notable changes to `nx9-auth` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-07-22

### Added
- **Unified Modular Runtime Lifecycle**: Fully implemented runtime subsystem (`Application`, `ApplicationBuilder`, `AtomicRuntimeState`, `SignalManager`, `ShutdownCoordinator`, `WorkerManager`, `HookRegistry`, `RuntimeMetrics`).
- **Axum HTTP Server Graceful Shutdown**: Integrated HTTP listener lifecycle with Tokio signal handling (`SIGINT` and `SIGTERM`).
- **Prioritized Shutdown Hooks**: Extensible shutdown hook execution (`First`, `Normal`, `Last`) with isolated failure handling.
- **Lock-Free State Machine**: Deterministic, lock-free lifecycle state transitions (`Initializing` -> `Starting` -> `Running` -> `Draining` -> `StoppingWorkers` -> `ExecutingHooks` -> `ClosingResources` -> `Stopped`).
- **Comprehensive Integration Tests**: Runtime lifecycle test suite verifying dependency assembly, hook order execution, and worker management.

### Changed
- Refactored `run_server` entrypoint in `main.rs` to construct and await the `Application` runtime lifecycle cleanly.
- Updated database connection pool closing to execute during the `ClosingResources` lifecycle phase.

### Fixed
- Fixed runtime completeness regression where `Application::start()` returned immediately instead of serving HTTP requests.
- Resolved database provider initialization lifecycle synchronization between CLI subcommands and server mode.
