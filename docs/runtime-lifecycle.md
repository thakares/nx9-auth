# Runtime Lifecycle Subsystem

The `nx9-auth` runtime lifecycle subsystem provides an enterprise-grade, lock-free, deterministic architecture for application startup, dependency assembly, operational observability, background worker coordination, prioritized shutdown hooks, and graceful HTTP server termination.

## Architecture Overview

```
CLI Commands / binary entrypoint (main.rs)
   │
   ▼
ApplicationBuilder
   │
   ├── Database Initialization (SQLite / PostgreSQL)
   ├── Repository Provider Assembly
   ├── AppState Construction
   └── Router Construction (Axum API + SPA UI)
   │
   ▼
Application Container (Lifecycle)
   │
   ├── AtomicRuntimeState Machine
   ├── SignalManager (SIGINT / SIGTERM)
   ├── ShutdownCoordinator (CancellationToken Hierarchy)
   ├── WorkerManager (Task Groups)
   ├── HookRegistry (Prioritized Shutdown Hooks)
   └── RuntimeMetrics
   │
   ▼
axum::serve (HTTP Server)
```

## Lifecycle States (`RuntimeState`)

The state machine is lock-free and driven by `AtomicU8` with `compare_exchange` transitions.

| State | Value | Description |
| :--- | :--- | :--- |
| `Initializing` | 0 | Runtime configuration loading and dependency assembly. |
| `Starting` | 1 | Database connection pool init, migrations, router assembly. |
| `Running` | 2 | HTTP server bound and actively serving requests. |
| `Draining` | 3 | Shutdown signal received; server stops accepting new connections, draining existing HTTP requests. |
| `StoppingWorkers` | 4 | Cancelling and joining active background worker tasks. |
| `ExecutingHooks` | 5 | Executing registered shutdown hooks in priority order (`First` -> `Normal` -> `Last`). |
| `ClosingResources` | 6 | Closing database connection pools and flushing logs. |
| `Stopped` | 7 | All resources released cleanly; runtime process exits with status 0. |

## Startup Sequence

1. `main()` parses CLI flags and loads configuration via `Config::find_and_load()`.
2. `run_server()` invokes `Application::builder(config).build().await`.
3. `ApplicationBuilder` creates `Application` and executes `initialize()`.
4. `initialize()` transitions state to `Starting`, connects database pool, executes migrations, and builds `Router`.
5. `app.start().await` transitions state to `Running`, binds `TcpListener`, prints `Listening on <addr>`, and awaits `axum::serve`.

## Graceful Shutdown Sequence

1. `SIGINT` (Ctrl+C) or `SIGTERM` signal received by `SignalManager` or `ShutdownCoordinator`.
2. `axum::serve` completes its graceful shutdown loop, stopping the TCP listener.
3. State transitions to `Draining`.
4. State transitions to `StoppingWorkers`; `WorkerManager` cancels and joins task groups.
5. State transitions to `ExecutingHooks`; `HookRegistry` executes registered hooks.
6. State transitions to `ClosingResources`; `PoolHandle` closes the database pool.
7. State transitions to `Stopped`; application returns `Ok(())` with exit status 0.
