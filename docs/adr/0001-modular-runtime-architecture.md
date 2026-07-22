# ADR 0001: Modular Runtime Architecture and State Machine

## Status
Accepted

## Context
Following an initial refactor, the application runtime lacked a unified lifecycle container capable of keeping the HTTP server process alive while coordinating background workers, signal handling, and connection pool teardown.

## Decision
We adopted a modular runtime architecture in `src/runtime/`:
1. `Application`: Application container implementing `Lifecycle` (`initialize`, `start`, `shutdown`).
2. `ApplicationBuilder`: Builder pattern separating dependency wiring from runtime logic.
3. `AtomicRuntimeState`: Lock-free `AtomicU8` state machine ensuring atomic state transitions.
4. `SignalManager` & `ShutdownCoordinator`: Signal routing and hierarchical cancellation.
5. `HookRegistry` & `WorkerManager`: Extensible shutdown hooks and worker task tracking.

## Consequences
- Clean separation of concern between CLI parsing, dependency resolution, HTTP serving, and shutdown logic.
- Zero risk of zombie processes or unclosed database connections on SIGINT/SIGTERM.
- Fully observable startup and shutdown transitions.
