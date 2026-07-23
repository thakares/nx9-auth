# nx9-auth

<p align="center">

**Enterprise Identity & Access Management (IAM)**

*Self-Hosted • Privacy-First • Pure Rust • Single Binary • SQLite & PostgreSQL*

[![Version](https://img.shields.io/badge/version-v0.3.0-blue.svg)]()
[![Rust](https://img.shields.io/badge/Rust-2024-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache2--0%20%7C%20MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux-success.svg)]()
[![SQLite](https://img.shields.io/badge/database-SQLite-blue.svg)]()
[![PostgreSQL](https://img.shields.io/badge/database-PostgreSQL-blue.svg)]()

</p>

---

## Overview

**nx9-auth** is a self-hosted Identity & Access Management (IAM) server built entirely in **Rust**. It provides centralized authentication, multi-tenancy, fine-grained Role-Based Access Control (RBAC), Personal Access Tokens (PATs), service accounts, application registration credentials, active session management, and append-only audit logging.

The server uses **Axum**, **Tokio**, and **SQLx**, with repository implementations for both embedded **SQLite** and external **PostgreSQL** deployments. Its administration interface is built with **Dioxus 0.6** and compiled to **WebAssembly (WASM)**, requiring no Node.js or npm runtime/build chain for the application architecture.

The runtime lifecycle and Application Registration Credentials subsystems have completed dedicated production-hardening passes covering deterministic shutdown, live signal escalation, transactional credential operations, secret handling, authorization boundaries, redirect URI validation, and regression testing.

---

## Key Features

- **Deterministic Runtime Lifecycle**: Atomic 8-state lifecycle (`Initializing` → `Starting` → `Running` → `Draining` → `StoppingWorkers` → `ExecutingHooks` → `ClosingResources` → `Stopped`) with graph-validated transitions, cancellation propagation, supervised worker shutdown, HTTP draining, prioritized shutdown hooks, and live forced escalation on a second Unix termination signal.
- **SQLite & PostgreSQL Support**: Shared repository abstraction with backend-specific migrations and repository implementations for embedded SQLite and external PostgreSQL deployments.
- **Security-Oriented Authentication**: Argon2id password hashing, BLAKE3 credential/token digests, constant-time credential comparison, rate limiting, non-enumerating authentication failures, secret redaction, and security response headers.
- **Multi-Tenant RBAC**: Tenant-aware identities, fine-grained permissions, role assignments, and organizational user groups.
- **Personal Access Tokens & Service Accounts**: Credentials for API and machine-to-machine access with hashed-at-rest secrets and revocation support.
- **Application Registration Credentials**: Immutable server-generated Client IDs, one-time Client Secret disclosure, BLAKE3 secret hashing, constant-time verification, secret rotation, redirect URI metadata, scopes, and dedicated `applications:manage` authorization.
- **Transactional Credential Integrity**: Application creation and Client Secret rotation are committed atomically with their audit records; audit failure rolls back the associated credential operation.
- **Strict Application Identity**: Application authentication uses `client_id` only; editable application slugs are never accepted as credential identities.
- **Redirect URI Policy**: Registered redirect URIs are structurally validated. HTTPS is supported generally; HTTP is restricted to localhost/loopback development destinations. Fragments, userinfo credentials, unsupported schemes, excessive URI counts, and oversized entries are rejected.
- **Embedded WebAssembly Administration UI**: Dioxus-powered administration interface compiled to WASM without a Node.js/React frontend stack.
- **Structured Auditability**: Security-sensitive lifecycle and identity operations are audit logged while plaintext passwords, Client Secrets, tokens, and stored credential hashes are excluded from audit metadata.
- **CLI Tooling**: Command-line workflows for initialization, diagnostics, migration, backup/restore, server operation, and identity administration.

---

## Application Registration Credentials

Applications are registered with a stable public identity and a high-entropy secret:

```text
Client ID:     nx9_app_<32 lowercase hex characters>
Client Secret: nx9_secret_<64 lowercase hex characters>
```

The **Client ID** is immutable and safe to identify an application. The **Client Secret** is disclosed only when the application is created or its secret is explicitly rotated.

Plaintext Client Secrets are never persisted. NX9-Auth stores a BLAKE3 digest and performs credential comparison using constant-time byte comparison. Creation and secret rotation responses are treated as one-time secret disclosure operations and use `Cache-Control: no-store`.

Existing applications upgraded from earlier schemas receive a stable Client ID. Applications without previously configured credentials can establish credentials through explicit secret rotation.

> **Protocol boundary:** Application credentials, redirect URIs, and scopes form the application registration layer. Redirect URIs are registration metadata intended to become security-enforced redirect destinations when OAuth2/OIDC protocol handlers are implemented. This registration subsystem does not by itself claim complete OAuth2/OIDC grant-flow support.

---

## Runtime Lifecycle

NX9-Auth uses an explicit lifecycle graph:

```mermaid
stateDiagram-v2
    [*] --> Initializing
    Initializing --> Starting: Build Runtime
    Starting --> Running: Start Services
    Running --> Draining: Begin Shutdown
    Draining --> StoppingWorkers: HTTP Drain Completes or Is Force-Aborted
    StoppingWorkers --> ExecutingHooks: Workers Terminated
    ExecutingHooks --> ClosingResources: Hooks Complete
    ClosingResources --> Stopped: Resources Closed
    Stopped --> [*]
```

The first `SIGINT` or `SIGTERM` initiates graceful shutdown, transitions the runtime into `Draining`, and begins HTTP request draining. Signal monitoring remains active throughout shutdown. A second termination signal escalates shutdown immediately, allowing HTTP draining and blocked worker waits to be curtailed rather than consuming the remaining graceful deadline.

Worker groups receive cancellation concurrently and operate under a shared global shutdown budget. Shutdown hooks execute deterministically by priority, with hooks at the same priority retaining registration order.

---

## Quickstart

```bash
# Initialize application directory, configuration, and default administrator
nx9-auth init

# Verify installation and system health
nx9-auth doctor

# Start server
nx9-auth serve
```

---

## Configuration

Configure `config.toml` or use the supported environment-variable configuration:

```toml
[server]
host = "127.0.0.1"
port = 8655
production = false
cookie_secure = false

[database]
# SQLite URL or file path:
url = "sqlite://./data/auth.db?mode=rwc"

# Or PostgreSQL:
# url = "postgres://user:password@localhost:5432/nx9auth"

max_connections = 20
min_connections = 5
connect_timeout_secs = 10
idle_timeout_secs = 600
max_lifetime_secs = 1800

[shutdown]
graceful_timeout_secs = 30
force_timeout_secs = 35
```

For production deployments, terminate TLS appropriately, use secure cookies, protect configuration and database credentials, and apply deployment-specific filesystem and network permissions.

---

## Security Model

NX9-Auth applies layered controls rather than relying on any single authentication mechanism:

| Area | Control |
|---|---|
| Passwords | Argon2id password hashing |
| Application secrets | BLAKE3 digest at rest |
| Credential comparison | `subtle::ConstantTimeEq` |
| Authentication failures | Non-enumerating unauthorized responses |
| Application mutations | Dedicated `applications:manage` permission |
| Application creation | Transactional application + audit insertion |
| Secret rotation | Transactional secret update + audit insertion |
| Secret disclosure | One-time response; never returned by list/GET operations |
| Redirect URIs | Structural and scheme-policy validation |
| HTTP responses | Security headers and no-store handling for secret responses |
| Audit metadata | Secret and credential-hash redaction |

Security controls documented here describe implemented mechanisms and should not be interpreted as a substitute for deployment-specific threat modelling, security review, or external audit.

---

## Verification

The runtime lifecycle and Application Registration Credentials hardening scopes are covered by workspace unit, integration, migration, security, and acceptance tests.

The verification gates used for these scopes are:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --release
cargo check --manifest-path ui/Cargo.toml --target wasm32-unknown-unknown
```

At the documented hardening checkpoint, the workspace test suite completed with **97 tests passed and 0 failed**. Test counts and execution times are verification-run observations rather than performance guarantees.

---

## Documentation Index

- [System Architecture](docs/ARCHITECTURE.md)
- [Runtime Lifecycle & Graceful Shutdown](docs/RUNTIME_LIFECYCLE.md)
- [Security Architecture](docs/SECURITY.md)
- [Release Notes](RELEASE_NOTES.md)
- [Authentication Model](docs/AUTHENTICATION.md)
- [Backup & Disaster Recovery](docs/BACKUPS.md)
- [Docker Deployment Guide](docs/DOCKER.md)
- [Linux Deployment Guide](docs/DEPLOYMENT.md)
- [Integration Guide](docs/INTEGRATION_BZOD.md)
- [Performance Benchmarks](docs/BENCHMARKS.md)
- [Changelog](CHANGELOG.md)
- [License](LICENSE)

---

## Project Status

The **Runtime Lifecycle** and **Application Registration Credentials** hardening scopes documented for the current release are complete.

Future OAuth2/OIDC protocol handlers, additional authentication protocols, deployment hardening, or architectural changes should be introduced as separately scoped work with corresponding migrations, security review, and regression tests.

---

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE](LICENSE) or <http://opensource.org/licenses/MIT>)

at your option.
