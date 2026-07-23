# Runtime Lifecycle Subsystem

> **Project:** NX9-Auth\
> **Release baseline:** v0.3.0\
> **Scope:** Application runtime, startup, serving, signals, workers,
> shutdown hooks, cancellation, metrics, and resource termination\
> **Primary implementation:** `src/runtime/`

------------------------------------------------------------------------

## 1. Purpose

The NX9-Auth runtime lifecycle subsystem provides the orchestration
boundary between the CLI entrypoint and the long-running HTTP
application. It coordinates configuration, dependency assembly, database
initialization, Axum router construction, listener binding, background
workers, operating-system signals, graceful shutdown, prioritized
cleanup hooks, runtime metrics, and final resource release.

The subsystem is intentionally explicit. Runtime state is represented by
a monotonic state machine rather than inferred from scattered booleans
or task state. Shutdown is coordinated rather than ad hoc, and worker
termination precedes final resource cleanup.

------------------------------------------------------------------------

## 2. Design Goals

The lifecycle architecture is designed around these goals:

1.  **Deterministic startup** --- dependencies are initialized in a
    defined order before serving traffic.
2.  **Deterministic shutdown** --- termination follows an explicit
    sequence from draining through resource closure.
3.  **Monotonic runtime state** --- normal lifecycle transitions move
    forward only.
4.  **Low synchronization overhead** --- lifecycle state uses atomic
    operations rather than a mutex.
5.  **Explicit ownership** --- the `Application` container owns or
    coordinates the principal runtime components.
6.  **Graceful HTTP termination** --- in-flight requests are given an
    opportunity to complete.
7.  **Bounded worker shutdown** --- background tasks are cancelled and
    joined with timeout handling.
8.  **Ordered cleanup** --- shutdown hooks execute by defined priority.
9.  **Signal-aware operation** --- normal termination signals initiate
    graceful shutdown; escalation can force termination.
10. **Testability** --- state transitions, worker handling, builder
    behavior, and hook ordering are independently testable.

------------------------------------------------------------------------

## 3. Architectural Principles

The runtime follows several engineering principles.

### 3.1 Explicit lifecycle over implicit lifecycle

Startup and shutdown are represented by named states. This makes
operational behavior observable and prevents lifecycle logic from being
distributed invisibly across unrelated modules.

### 3.2 Construction before exposure

The HTTP server is not considered running until configuration, database
initialization, migrations, application state, and router assembly have
completed.

### 3.3 Cancellation before destruction

Background work is asked to terminate before resources it may depend on
are released.

### 3.4 Ordered cleanup

Cleanup operations with dependencies can be expressed through
shutdown-hook priority instead of relying on incidental registration
order.

### 3.5 Idempotent shutdown initiation

Multiple shutdown triggers must not cause multiple independent teardown
sequences.

------------------------------------------------------------------------

## 4. High-Level Architecture

``` mermaid
flowchart TD
    CLI["CLI entrypoint"] --> CFG["Load configuration"]
    CFG --> BUILDER["ApplicationBuilder"]
    BUILDER --> DB["Initialize database"]
    DB --> MIG["Run migrations"]
    MIG --> STATE["Build AppState"]
    STATE --> ROUTER["Build Axum router"]
    ROUTER --> APP["Application"]
    APP --> LISTENER["Bind TCP listener"]
    LISTENER --> SERVER["Axum HTTP server"]
    APP --> WORKERS["WorkerManager"]
    APP --> SIGNALS["SignalManager"]
    APP --> CANCEL["ShutdownCoordinator"]
    APP --> HOOKS["HookRegistry"]
    APP --> METRICS["RuntimeMetrics"]
```

This diagram deliberately uses simple Mermaid labels so that it renders
consistently on GitHub.

------------------------------------------------------------------------

## 5. Runtime Component Map

  -------------------------------------------------------------------------------
  Component               Primary location                Responsibility
  ----------------------- ------------------------------- -----------------------
  `Application`           `src/runtime/application.rs`    Central lifecycle
                                                          container and
                                                          orchestration

  `ApplicationBuilder`    `src/runtime/builder.rs`        Dependency assembly and
                                                          application
                                                          initialization

  `AtomicRuntimeState`    `src/runtime/state.rs`          Lock-free lifecycle
                                                          state tracking

  `SignalManager`         `src/runtime/signals.rs`        SIGINT/SIGTERM handling
                                                          and escalation

  `ShutdownCoordinator`   `src/runtime/cancellation.rs`   Cancellation
                                                          propagation

  `WorkerManager`         `src/runtime/workers.rs`        Background task
                                                          grouping, tracking, and
                                                          termination

  `HookRegistry`          `src/runtime/hooks.rs`          Prioritized
                                                          asynchronous shutdown
                                                          hooks

  `RuntimeMetrics`        `src/runtime/metrics.rs`        Runtime-level
                                                          operational counters

  `Lifecycle`             `src/runtime/lifecycle.rs`      Lifecycle abstraction
                                                          implemented by the
                                                          application
  -------------------------------------------------------------------------------

------------------------------------------------------------------------

## 6. Ownership Model

`Application` acts as the lifecycle ownership boundary.

``` mermaid
flowchart TD
    APP["Application"]
    APP --> CONFIG["Config"]
    APP --> PROVIDER["Repository provider"]
    APP --> POOL["Database pool handle"]
    APP --> ROUTER["Axum router"]
    APP --> STATE["AtomicRuntimeState"]
    APP --> WORKERS["WorkerManager"]
    APP --> SIGNALS["SignalManager"]
    APP --> CANCEL["ShutdownCoordinator"]
    APP --> HOOKS["HookRegistry"]
    APP --> METRICS["RuntimeMetrics"]
```

The model avoids global mutable lifecycle state. Shared runtime
coordination is performed through purpose-specific primitives such as
atomics, cancellation tokens, task sets, and shared application state.

------------------------------------------------------------------------

## 7. Dependency Assembly

The builder separates construction from execution.

``` mermaid
sequenceDiagram
    participant Main
    participant Builder
    participant Database
    participant State
    participant Router
    participant App

    Main->>Builder: build
    Builder->>Database: initialize provider
    Database-->>Builder: provider and pool
    Builder->>Database: run migrations
    Builder->>State: construct AppState
    State-->>Builder: application state
    Builder->>Router: build router
    Router-->>Builder: router
    Builder->>App: assemble runtime
    App-->>Main: initialized application
```

This boundary is important because the runtime can fail during
construction without exposing a partially serving HTTP process.

------------------------------------------------------------------------

## 8. Lifecycle State Model

NX9-Auth defines the following lifecycle states.

  ------------------------------------------------------------------------
  State                                        Value Meaning
  --------------------- ---------------------------- ---------------------
  `Initializing`                                   0 Configuration and
                                                     initial runtime
                                                     construction

  `Starting`                                       1 Database, migrations,
                                                     state, router, and
                                                     server preparation

  `Running`                                        2 HTTP listener is
                                                     active and the
                                                     service is
                                                     operational

  `Draining`                                       3 Shutdown has started
                                                     and HTTP traffic is
                                                     being drained

  `StoppingWorkers`                                4 Background workers
                                                     are being cancelled
                                                     and joined

  `ExecutingHooks`                                 5 Registered shutdown
                                                     hooks are executing

  `ClosingResources`                               6 Long-lived resources
                                                     are being released

  `Stopped`                                        7 Lifecycle termination
                                                     is complete
  ------------------------------------------------------------------------

------------------------------------------------------------------------

## 9. Deterministic State Machine

``` mermaid
stateDiagram-v2
    [*] --> Initializing
    Initializing --> Starting
    Starting --> Running
    Running --> Draining
    Draining --> StoppingWorkers
    StoppingWorkers --> ExecutingHooks
    ExecutingHooks --> ClosingResources
    ClosingResources --> Stopped
    Stopped --> [*]
```

Normal transitions are strictly forward. A runtime state must not
regress from a later lifecycle phase to an earlier phase.

------------------------------------------------------------------------

## 10. Runtime State Invariants

The lifecycle design is intended to preserve these invariants:

-   `Running` cannot be reached before startup initialization succeeds.
-   Normal state transitions are monotonic.
-   Shutdown initiation is guarded against duplicate execution.
-   Worker termination occurs before final resource closure.
-   Shutdown hooks execute after worker shutdown begins and before final
    resource release.
-   `Stopped` represents completion of the lifecycle sequence.
-   Emergency state advancement is explicit rather than silently
    violating transition rules.

------------------------------------------------------------------------

## 11. Atomic Runtime State

`AtomicRuntimeState` stores lifecycle state using `AtomicU8`.

The compact representation is suitable because the state space is small
and fixed. Atomic operations allow state inspection and transitions
without introducing a mutex solely for lifecycle coordination.

The implementation uses atomic memory ordering, including `Acquire` and
`AcqRel`, to coordinate visibility of lifecycle changes between
asynchronous execution contexts.

### Why an atomic state machine?

-   State reads are inexpensive.
-   Transition attempts can use compare-and-exchange semantics.
-   Duplicate transition attempts can be detected.
-   Shutdown initiation can be guarded without a global lock.
-   The state machine remains independent of Tokio scheduling.

`force_set()` exists for exceptional or escalation paths and should not
replace validated normal transitions.

------------------------------------------------------------------------

## 12. Startup Lifecycle

The normal startup path is:

``` mermaid
flowchart TD
    A["Process starts"] --> B["Parse CLI"]
    B --> C["Load configuration"]
    C --> D["Create ApplicationBuilder"]
    D --> E["Initialize database provider"]
    E --> F["Run migrations"]
    F --> G["Construct AppState"]
    G --> H["Construct Axum router"]
    H --> I["Transition to Starting"]
    I --> J["Bind TCP listener"]
    J --> K["Transition to Running"]
    K --> L["Serve HTTP"]
```

------------------------------------------------------------------------

## 13. Startup Sequence in Detail

### Phase 1 --- Entrypoint

The binary entrypoint parses CLI arguments. The `serve` command enters
the long-running server path.

### Phase 2 --- Configuration

Configuration is discovered and loaded before runtime dependencies are
assembled.

A configuration failure prevents startup.

### Phase 3 --- Application construction

`Application::builder(config).build().await` creates the runtime
container and initializes dependencies.

### Phase 4 --- Database initialization

The configured SQLite or PostgreSQL backend is initialized through the
database provider abstraction.

Pending migrations are applied before the service begins serving
requests.

### Phase 5 --- Application state

Database and repository dependencies are incorporated into `AppState`.

### Phase 6 --- Router construction

The Axum API and SPA routes are assembled using the initialized
application state.

### Phase 7 --- Listener binding

The runtime binds a `TcpListener` to the configured host and port.

Listener binding is a startup boundary: a bind failure must abort
startup rather than incorrectly reporting `Running`.

### Phase 8 --- Serving

After successful binding, the runtime transitions to `Running` and
executes `axum::serve` with graceful shutdown integration.

------------------------------------------------------------------------

## 14. Startup Failure Boundaries

``` mermaid
flowchart TD
    START["Startup"] --> CFG{"Configuration valid"}
    CFG -- No --> FAIL["Return startup error"]
    CFG -- Yes --> DB{"Database initialized"}
    DB -- No --> FAIL
    DB -- Yes --> MIG{"Migrations successful"}
    MIG -- No --> FAIL
    MIG -- Yes --> ROUTER{"Router constructed"}
    ROUTER -- No --> FAIL
    ROUTER -- Yes --> BIND{"Listener bound"}
    BIND -- No --> FAIL
    BIND -- Yes --> RUN["Running"]
```

A failed prerequisite must not leave the application falsely marked as
operational.

------------------------------------------------------------------------

## 15. HTTP Server Lifecycle

The HTTP server is created only after dependency assembly.

``` mermaid
flowchart LR
    LISTENER["TcpListener"] --> AXUM["axum serve"]
    ROUTER["Router"] --> AXUM
    SHUTDOWN["Graceful shutdown future"] --> AXUM
    AXUM --> REQUESTS["HTTP requests"]
```

The graceful shutdown future connects external termination events to
Axum's server-draining behavior.

------------------------------------------------------------------------

## 16. Shutdown Architecture

Shutdown is not a single cancellation call. It is a sequence of
lifecycle phases.

``` mermaid
flowchart TD
    SIGNAL["Shutdown trigger"] --> GUARD["Initiate shutdown once"]
    GUARD --> CANCEL["Cancel root token"]
    CANCEL --> DRAIN["Drain HTTP server"]
    DRAIN --> STOP["Stop workers"]
    STOP --> HOOKS["Execute shutdown hooks"]
    HOOKS --> CLOSE["Close resources"]
    CLOSE --> DONE["Stopped"]
```

------------------------------------------------------------------------

## 17. Shutdown Triggers

Graceful shutdown may originate from:

-   `SIGINT`
-   `SIGTERM`
-   programmatic cancellation through the shutdown coordinator
-   another controlled runtime termination path

The first shutdown event initiates graceful termination.

A subsequent signal can be treated as escalation when graceful shutdown
is no longer appropriate.

------------------------------------------------------------------------

## 18. Signal Management

`SignalManager` is responsible for translating operating-system
termination signals into runtime shutdown events.

``` mermaid
sequenceDiagram
    participant OS
    participant Signals
    participant Runtime
    participant Server

    OS->>Signals: first termination signal
    Signals->>Runtime: initiate graceful shutdown
    Runtime->>Server: begin draining
    OS->>Signals: second termination signal
    Signals->>Runtime: escalate termination
```

The distinction between first-signal graceful termination and subsequent
escalation prevents a stuck cleanup operation from indefinitely blocking
process termination.

------------------------------------------------------------------------

## 19. Cancellation Architecture

The shutdown coordinator uses `tokio_util::sync::CancellationToken`.

``` mermaid
flowchart TD
    ROOT["Root cancellation token"]
    ROOT --> HTTP["HTTP shutdown"]
    ROOT --> WG1["Worker group A"]
    ROOT --> WG2["Worker group B"]
    ROOT --> AUX["Auxiliary subsystem"]
```

Child cancellation tokens provide hierarchical propagation while
allowing subsystems to respond independently.

Cancellation is a request to stop. It does not by itself prove that a
task has terminated; worker joining remains necessary.

------------------------------------------------------------------------

## 20. Worker Management

`WorkerManager` groups asynchronous tasks and tracks their lifecycle.

The implementation uses `tokio::task::JoinSet` for task coordination.

Responsibilities include:

-   registering background work,
-   grouping related tasks,
-   propagating cancellation,
-   waiting for worker completion,
-   enforcing bounded shutdown,
-   aborting tasks that exceed the permitted shutdown window.

------------------------------------------------------------------------

## 21. Worker Shutdown

``` mermaid
flowchart TD
    A["StoppingWorkers"] --> B["Signal cancellation"]
    B --> C["Wait for worker completion"]
    C --> D{"Completed before timeout"}
    D -- Yes --> E["Worker group stopped"]
    D -- No --> F["Abort remaining tasks"]
    F --> E
    E --> G["ExecutingHooks"]
```

Bounded shutdown is essential. Graceful termination must not become an
unlimited wait on a non-responsive task.

------------------------------------------------------------------------

## 22. Worker Hierarchy

``` mermaid
flowchart TD
    WM["WorkerManager"]
    WM --> G1["TaskGroup A"]
    WM --> G2["TaskGroup B"]
    G1 --> T1["Task 1"]
    G1 --> T2["Task 2"]
    G2 --> T3["Task 3"]
```

Task grouping allows cancellation and joining to be reasoned about at
subsystem boundaries rather than as unrelated spawned tasks.

------------------------------------------------------------------------

## 23. Shutdown Hooks

The runtime supports asynchronous shutdown hooks through the
`ShutdownHook` abstraction.

``` rust
#[async_trait::async_trait]
pub trait ShutdownHook: Send + Sync {
    fn name(&self) -> &'static str;

    fn priority(&self) -> ShutdownPriority {
        ShutdownPriority::Normal
    }

    async fn shutdown(&self) -> Result<()>;
}
```

Hooks are intended for cleanup operations that must participate in
lifecycle ordering.

------------------------------------------------------------------------

## 24. Hook Priorities

The defined order is:

1.  `First`
2.  `Normal`
3.  `Last`

``` mermaid
flowchart LR
    FIRST["First"] --> NORMAL["Normal"]
    NORMAL --> LAST["Last"]
```

Priority is semantic. A cleanup dependency should be represented by the
appropriate priority rather than relying on accidental registration
order.

------------------------------------------------------------------------

## 25. Hook Execution Model

``` mermaid
sequenceDiagram
    participant Runtime
    participant Registry
    participant First
    participant Normal
    participant Last

    Runtime->>Registry: execute hooks
    Registry->>First: shutdown
    First-->>Registry: complete
    Registry->>Normal: shutdown
    Normal-->>Registry: complete
    Registry->>Last: shutdown
    Last-->>Registry: complete
    Registry-->>Runtime: hook phase complete
```

A robust shutdown implementation should preserve cleanup progression
even when an individual hook reports an error, while ensuring failures
remain observable.

------------------------------------------------------------------------

## 26. Resource Closure

The `ClosingResources` phase follows worker shutdown and hook execution.

Resources may include:

-   database connection pools,
-   repository/provider state,
-   runtime-owned communication primitives,
-   logging or telemetry buffers,
-   other long-lived infrastructure owned by the application.

The ordering prevents dependent background work from using resources
after they have been destroyed.

------------------------------------------------------------------------

## 27. Complete Shutdown Sequence

``` mermaid
sequenceDiagram
    participant Signal
    participant State
    participant Server
    participant Workers
    participant Hooks
    participant Resources

    Signal->>State: initiate shutdown
    State->>State: Running to Draining
    State->>Server: graceful shutdown
    Server-->>State: HTTP drained
    State->>State: Draining to StoppingWorkers
    State->>Workers: cancel and join
    Workers-->>State: workers stopped
    State->>State: StoppingWorkers to ExecutingHooks
    State->>Hooks: execute by priority
    Hooks-->>State: hooks complete
    State->>State: ExecutingHooks to ClosingResources
    State->>Resources: close
    Resources-->>State: released
    State->>State: ClosingResources to Stopped
```

------------------------------------------------------------------------

## 28. Shutdown Failure Strategy

Shutdown differs from startup failure handling.

During startup, a failed mandatory prerequisite normally aborts startup.

During shutdown, a cleanup failure should generally be recorded while
allowing later cleanup phases to proceed where safe. Otherwise, one
failed hook could prevent unrelated resources from being released.

Important failure categories include:

-   worker timeout,
-   worker panic,
-   shutdown hook error,
-   resource-close error,
-   signal escalation,
-   cancellation races.

The exact error policy should remain explicit in implementation and
tests.

------------------------------------------------------------------------

## 29. Concurrency Model

The lifecycle subsystem is built on Tokio asynchronous execution.

Primary concurrency mechanisms include:

-   Tokio tasks,
-   `JoinSet`,
-   `CancellationToken`,
-   atomic lifecycle state,
-   shared application state,
-   asynchronous shutdown hooks.

The lifecycle state machine itself does not depend on a blocking mutex.

------------------------------------------------------------------------

## 30. Thread-Safety Model

Runtime coordination should avoid mutable global state.

Cross-task lifecycle communication is expressed through primitives
designed for concurrent access:

``` mermaid
flowchart LR
    TASK1["Task A"] --> ATOMIC["Atomic state"]
    TASK2["Task B"] --> ATOMIC
    SIGNAL["Signal task"] --> TOKEN["Cancellation token"]
    TOKEN --> TASK1
    TOKEN --> TASK2
```

Atomic state answers the question "which lifecycle phase are we in?"

Cancellation answers the question "should this work stop?"

Task joining answers the question "has this work actually stopped?"

These are separate concerns and should remain separate.

------------------------------------------------------------------------

## 31. Memory Ordering

The state implementation uses atomic memory ordering such as `Acquire`
and `AcqRel`.

At a conceptual level:

-   **Acquire** prevents subsequent operations from being reordered
    before an observed synchronization point.
-   **Release** publishes prior operations before a synchronization
    point.
-   **AcqRel** combines both properties for read-modify-write
    operations.

The runtime should use the weakest ordering that still preserves its
required synchronization semantics, but lifecycle correctness takes
priority over micro-optimization.

Changes to atomic ordering require careful review because a state
transition is part of inter-task coordination rather than merely a
numeric assignment.

------------------------------------------------------------------------

## 32. Why `AtomicU8`

The lifecycle has a small finite state space, making an integer-backed
atomic representation appropriate.

Benefits include:

-   compact storage,
-   constant-time reads,
-   compare-and-exchange transitions,
-   no lifecycle mutex contention,
-   explicit conversion between stored values and semantic states.

The numeric representation is an implementation detail. Callers should
reason in terms of `RuntimeState`, not raw integers.

------------------------------------------------------------------------

## 33. Observability

Runtime transitions should be visible in structured logs.

A typical startup trace includes:

``` text
Initializing
Starting
Running
Listening on 0.0.0.0:8655
```

Structured transition logging should identify both the previous and next
state.

This allows operators to distinguish:

-   configuration failure,
-   database startup failure,
-   listener failure,
-   successful service readiness,
-   graceful shutdown,
-   worker shutdown delays,
-   final termination.

------------------------------------------------------------------------

## 34. Runtime Metrics

`RuntimeMetrics` provides a lifecycle-level location for operational
measurements such as:

-   request counts,
-   error counts,
-   active worker counts.

Metrics should describe runtime behavior without becoming an alternate
source of lifecycle truth. `AtomicRuntimeState` remains authoritative
for lifecycle phase.

------------------------------------------------------------------------

## 35. Lifecycle and Request Processing

``` mermaid
flowchart TD
    RUN["Running"] --> LISTENER["TCP listener"]
    LISTENER --> ROUTER["Axum router"]
    ROUTER --> MW["Middleware"]
    MW --> HANDLER["API or UI handler"]
    HANDLER --> RESPONSE["HTTP response"]
    SIGNAL["Shutdown signal"] --> DRAIN["Draining"]
    DRAIN --> LISTENER
```

During normal operation, requests flow through the listener and router.

When graceful shutdown begins, the server stops accepting new work
according to Axum's graceful shutdown behavior and allows in-flight
requests to complete before later teardown phases proceed.

------------------------------------------------------------------------

## 36. Database Lifecycle

Database initialization is part of startup, not lazy runtime discovery.

``` mermaid
flowchart LR
    CONFIG["Database configuration"] --> INIT["Initialize provider"]
    INIT --> MIG["Run migrations"]
    MIG --> POOL["Pool ready"]
    POOL --> STATE["AppState"]
    STATE --> RUN["Running"]
```

On shutdown, database resources are retained until worker and hook
phases that may depend on them have completed.

------------------------------------------------------------------------

## 37. Router Lifecycle

The router is assembled after its required state is available.

This provides a clean dependency direction:

``` text
configuration
    -> database/provider
    -> AppState
    -> router
    -> listener/server
```

The HTTP layer therefore consumes initialized domain infrastructure
instead of being responsible for constructing it.

------------------------------------------------------------------------

## 38. ApplicationBuilder Responsibilities

`ApplicationBuilder` should remain focused on dependency assembly.

Its responsibilities include:

-   receiving validated runtime configuration,
-   initializing database infrastructure,
-   assembling provider/repository dependencies,
-   constructing `AppState`,
-   constructing the Axum router,
-   returning an initialized `Application`.

It should not become a second application runtime.

------------------------------------------------------------------------

## 39. Application Responsibilities

`Application` owns the execution lifecycle after construction.

Its responsibilities include:

-   state transitions,
-   listener binding,
-   HTTP serving,
-   graceful shutdown integration,
-   worker coordination,
-   shutdown-hook execution,
-   resource closure,
-   lifecycle metrics and observability.

This division keeps construction and execution conceptually distinct.

------------------------------------------------------------------------

## 40. Lifecycle Trait

The lifecycle abstraction exposes the major phases through operations
such as:

``` text
initialize
start
shutdown
```

The abstraction makes lifecycle behavior testable independently from the
CLI command parser.

The CLI should initiate lifecycle operations rather than duplicate their
implementation.

------------------------------------------------------------------------

## 41. Security Considerations

Lifecycle correctness contributes directly to security.

### 41.1 Controlled startup

The service must not report itself as operational before mandatory
security-sensitive dependencies are ready.

### 41.2 Controlled shutdown

Sessions, audit operations, database work, and background tasks should
not be abandoned arbitrarily during ordinary termination.

### 41.3 No duplicate teardown

Duplicate shutdown sequences can cause double-close behavior,
inconsistent logging, or races between cleanup operations.

### 41.4 Bounded termination

An attacker or malformed task should not be able to prevent process
termination indefinitely by blocking graceful worker shutdown.

### 41.5 Signal escalation

Forced termination provides an operational escape path when graceful
shutdown cannot complete.

------------------------------------------------------------------------

## 42. Performance Characteristics

The lifecycle subsystem is not expected to dominate request-path
performance.

Approximate structural complexity:

  Operation                     Characteristic
  ----------------------------- ----------------------------------------
  Runtime state read            O(1)
  Atomic transition             O(1)
  Cancellation propagation      proportional to cancellation tree
  Worker shutdown               O(n) relative to tracked tasks
  Hook execution                O(n) relative to registered hooks
  Startup dependency assembly   proportional to initialized components

Database initialization and migration cost dominate lifecycle startup
more than the state machine itself.

------------------------------------------------------------------------

## 43. Graceful Shutdown Timing

Shutdown timing should be treated as bounded operational policy.

``` mermaid
flowchart LR
    T0["Shutdown requested"] --> T1["HTTP drain"]
    T1 --> T2["Worker timeout window"]
    T2 --> T3["Hooks"]
    T3 --> T4["Resource close"]
    T4 --> T5["Exit"]
```

Timeout values should be configuration-driven where operational
requirements justify it.

A timeout is not an error by itself; it is a boundary after which the
runtime may need to escalate from graceful waiting to forced task
abortion.

------------------------------------------------------------------------

## 44. Extension Model

New runtime subsystems should integrate through existing lifecycle
primitives rather than creating independent shutdown mechanisms.

A new subsystem should determine:

1.  Who owns it?
2.  When is it initialized?
3.  Does it spawn background work?
4.  Which cancellation token does it observe?
5.  Must its tasks be joined?
6.  Does it require a shutdown hook?
7.  Which hook priority is correct?
8.  Which resources must remain alive until it stops?
9.  Which metrics and logs expose its lifecycle?

------------------------------------------------------------------------

## 45. Example Custom Shutdown Hook

``` rust
use nx9_auth::runtime::{ShutdownHook, ShutdownPriority};

struct CustomCleanupHook;

#[async_trait::async_trait]
impl ShutdownHook for CustomCleanupHook {
    fn name(&self) -> &'static str {
        "custom_cleanup"
    }

    fn priority(&self) -> ShutdownPriority {
        ShutdownPriority::First
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        println!("Flushing custom buffer before shutdown...");
        Ok(())
    }
}
```

Registration belongs in application assembly where lifecycle ownership
is visible.

------------------------------------------------------------------------

## 46. Example Runtime Bootstrap

``` rust
use nx9_auth::config::Config;
use nx9_auth::runtime::{Application, Lifecycle};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::find_and_load()?;
    let mut app = Application::builder(config).build().await?;
    app.start().await?;
    Ok(())
}
```

The actual binary may perform additional CLI handling, logging
initialization, or command dispatch around this lifecycle.

------------------------------------------------------------------------

## 47. Testing Strategy

The lifecycle subsystem is verified by
`tests/runtime_lifecycle_test.rs`.

The documented test areas include:

-   application builder lifecycle,
-   state transitions,
-   shutdown hook execution order,
-   worker manager lifecycle.

The broader NX9-Auth test suite additionally exercises application
behavior that depends on successful runtime assembly.

------------------------------------------------------------------------

## 48. Lifecycle Test Model

``` mermaid
flowchart TD
    TESTS["Runtime lifecycle tests"]
    TESTS --> BUILDER["Builder initialization"]
    TESTS --> STATES["State transitions"]
    TESTS --> HOOKS["Hook ordering"]
    TESTS --> WORKERS["Worker lifecycle"]
```

The tests should verify behavior rather than merely implementation
structure.

For example, hook tests should verify observable execution order instead
of assuming internal container ordering.

------------------------------------------------------------------------

## 49. Recommended Additional Tests

The following are valuable future lifecycle tests if not already
present:

-   invalid state transition rejection,
-   duplicate shutdown initiation,
-   worker timeout followed by abort,
-   shutdown-hook failure while later hooks still execute,
-   listener bind failure,
-   migration failure during initialization,
-   second-signal escalation,
-   cancellation propagation to child task groups,
-   shutdown with zero workers,
-   shutdown with zero hooks,
-   repeated state reads during concurrent shutdown.

These are recommendations, not claims about the current v0.3.0 test
suite.

------------------------------------------------------------------------

## 50. Operational Diagnostics

When investigating startup problems, check the last lifecycle state
reached.

### Stops during `Initializing`

Investigate configuration discovery and initial runtime construction.

### Stops during `Starting`

Investigate:

-   database connection,
-   migrations,
-   application state assembly,
-   router construction,
-   listener binding.

### Reaches `Running` but does not serve

Investigate:

-   bound address,
-   listener configuration,
-   router behavior,
-   reverse proxy configuration,
-   firewall/network path.

### Hangs during `StoppingWorkers`

Investigate worker cancellation handling and timeout behavior.

### Hangs during `ExecutingHooks`

Investigate shutdown hooks and external dependencies used by cleanup
operations.

### Hangs during `ClosingResources`

Investigate database and other long-lived resource termination.

------------------------------------------------------------------------

## 51. Logging Recommendations

Lifecycle logs should be:

-   structured,
-   concise,
-   emitted at state boundaries,
-   free of credentials and secrets,
-   sufficient to reconstruct startup and shutdown progression.

Recommended fields include:

``` text
event
from_state
to_state
component
duration
error
```

Sensitive configuration values must never be included merely to improve
lifecycle diagnostics.

------------------------------------------------------------------------

## 52. Anti-Patterns

### 52.1 Detached unmanaged tasks

Avoid spawning long-running tasks that are invisible to `WorkerManager`.

### 52.2 Independent shutdown flags

Avoid subsystem-specific boolean shutdown flags when cancellation tokens
already express the lifecycle signal.

### 52.3 Closing resources before workers

Workers may still require those resources.

### 52.4 Blocking indefinitely

Graceful shutdown requires bounded waits.

### 52.5 State regression

Do not move from a later lifecycle state back to an earlier state to
represent retries.

### 52.6 Using `force_set()` as normal control flow

Forced state mutation is an exceptional mechanism.

### 52.7 Hiding startup failure

Do not transition to `Running` if listener binding or mandatory
initialization failed.

------------------------------------------------------------------------

## 53. Review Checklist for Runtime Changes

Before merging changes to `src/runtime/`, verify:

-   [ ] lifecycle state transitions remain valid,
-   [ ] no state regression was introduced,
-   [ ] shutdown remains single-initiation,
-   [ ] new workers observe cancellation,
-   [ ] workers are tracked and joined,
-   [ ] shutdown waits are bounded,
-   [ ] resources outlive dependent workers,
-   [ ] hook ordering remains deterministic,
-   [ ] signal handling still supports graceful termination,
-   [ ] new logs contain no sensitive information,
-   [ ] lifecycle tests cover new behavior,
-   [ ] `cargo fmt` passes,
-   [ ] `cargo clippy` passes with project warning policy,
-   [ ] workspace tests pass.

------------------------------------------------------------------------

## 54. Source Layout

``` text
src/runtime/
├── application.rs
├── builder.rs
├── cancellation.rs
├── hooks.rs
├── lifecycle.rs
├── metrics.rs
├── mod.rs
├── signals.rs
├── state.rs
└── workers.rs
```

The exact module set should remain aligned with the repository as the
runtime evolves.

------------------------------------------------------------------------

## 55. Related Source Areas

The lifecycle subsystem interacts with:

``` text
src/main.rs
src/config/
src/db/
src/api/
src/middleware/
tests/runtime_lifecycle_test.rs
```

The runtime coordinates these areas but should not absorb their domain
responsibilities.

------------------------------------------------------------------------

## 56. Architecture Summary

``` mermaid
flowchart TD
    ENTRY["CLI"] --> BUILD["Build application"]
    BUILD --> READY["Dependencies ready"]
    READY --> RUN["Running"]
    RUN --> SIGNAL["Shutdown trigger"]
    SIGNAL --> DRAIN["Draining"]
    DRAIN --> WORKERS["Stopping workers"]
    WORKERS --> HOOKS["Executing hooks"]
    HOOKS --> RESOURCES["Closing resources"]
    RESOURCES --> STOP["Stopped"]
```

NX9-Auth's lifecycle architecture is centered on a simple rule:
**construction, operation, and destruction are explicit phases with
explicit ownership**.

The state machine makes those phases observable. Cancellation tokens
propagate intent. Worker management proves task termination. Hook
priorities provide deterministic cleanup ordering. Resource closure
occurs only after dependent work has been stopped.

------------------------------------------------------------------------

## 57. Verification Commands

Runtime lifecycle tests:

``` bash
cargo test --test runtime_lifecycle_test
```

Complete project test suite:

``` bash
cargo test --all-features
```

Formatting:

``` bash
cargo fmt --all -- --check
```

Clippy:

``` bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Release build:

``` bash
cargo build --release
```

For the WebAssembly administration UI, use the project's dedicated WASM
build process rather than assuming a native host build is equivalent.

------------------------------------------------------------------------

## 58. Document Maintenance

Update this document whenever a change modifies:

-   lifecycle states,
-   state transition rules,
-   startup ordering,
-   shutdown ordering,
-   signal semantics,
-   worker management,
-   cancellation hierarchy,
-   hook priority semantics,
-   resource ownership,
-   runtime metrics,
-   lifecycle test guarantees.

Architecture documentation should describe implemented behavior.
Proposed behavior should be explicitly identified as a recommendation or
future design rather than presented as current implementation.

------------------------------------------------------------------------

## 59. Status

For the v0.3.0 architecture baseline, the runtime lifecycle consists of
explicit application construction, atomic lifecycle state tracking,
signal-driven graceful shutdown, hierarchical cancellation, managed
background workers, prioritized shutdown hooks, runtime metrics, and
final resource cleanup.

The lifecycle progression is:

``` text
Initializing
    -> Starting
    -> Running
    -> Draining
    -> StoppingWorkers
    -> ExecutingHooks
    -> ClosingResources
    -> Stopped
```

This sequence forms the operational backbone of the NX9-Auth server
runtime.
