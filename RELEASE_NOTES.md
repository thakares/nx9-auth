# NX9-Auth v0.3.0 Release Notes

NX9-Auth v0.3.0 brings full architectural stabilization, unified runtime lifecycle management, and production-grade operational robustness to self-hosted Identity and Access Management.

## Key Features & Highlights

### ⚡ Unified Modular Runtime Subsystem
- **Application Container & Builder**: Pure dependency assembly separating configuration, database provider initializations, repository traits, and router construction.
- **Lock-Free State Machine**: `AtomicRuntimeState` tracks granular lifecycle states without mutex contention.
- **Signal Handling & Cancellation**: Multi-signal Unix signal manager handling `SIGINT` (Ctrl+C) and `SIGTERM` with parent-child cancellation tokens.

### 🛡️ Operational Stability & Graceful Shutdown
- **Orderly Shutdown Flow**: `Running` -> `Draining` -> `StoppingWorkers` -> `ExecutingHooks` -> `ClosingResources` -> `Stopped`.
- **Prioritized Hook Execution**: Supports custom shutdown hooks executed in priority order with error isolation.
- **Background Worker Management**: `WorkerManager` manages background task groups with configurable timeout cancellation.

### 🗄️ Dual-Database Engine Support
- Native support for SQLite (WAL mode, foreign keys, busy timeout) and PostgreSQL with automatic migrations and robust connection retry policies.

### 🚀 Developer & Operator Experience
- Built-in single binary execution (`nx9-auth serve`).
- Diagnostic `nx9-auth doctor` command for environment verification.
- Full Admin SPA UI shell embedded directly in the single binary.
