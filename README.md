# nx9-auth

<p align="center">

**Enterprise Identity & Access Management (IAM)**

*Self-Hosted • Privacy-First • Pure Rust • Single Binary • Dual Database Engine*

[![Version](https://img.shields.io/badge/version-v0.3.0-blue.svg)]()
[![Rust](https://img.shields.io/badge/Rust-2024-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache2--0%20%7C%20MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux-success.svg)]()
[![SQLite](https://img.shields.io/badge/database-SQLite-blue.svg)]()
[![PostgreSQL](https://img.shields.io/badge/database-PostgreSQL-blue.svg)]()

</p>

---

## Overview

**nx9-auth** is a production-grade, self-hosted Identity & Access Management (IAM) server built entirely in **Rust**. It provides multi-tenant user authentication, Role-Based Access Control (RBAC), Personal Access Tokens (PATs), OAuth2 service accounts, active session management, full audit logging, an enterprise graceful shutdown runtime lifecycle, and an embedded WebAssembly (WASM) administrative UI.

`nx9-auth` compiles into a single standalone binary containing both the Axum REST API backend and the embedded Dioxus WASM frontend, backed by a database-agnostic provider supporting both **SQLite** and **PostgreSQL**.

---

## Key Features

- **Unified Enterprise Runtime Lifecycle**: Atomic 8-state lifecycle machine (`Initializing` → `Starting` → `Running` → `Draining` → `StoppingWorkers` → `ExecutingHooks` → `ClosingResources` → `Stopped`), `CancellationToken` propagation, `JoinSet` worker management, prioritized shutdown hooks, and destructor-safe Unix signal escalation.
- **Dual Database Engine**: Native support for SQLite and enterprise PostgreSQL with 100% repository parity and runtime connection pool ownership.
- **Enterprise Security Model**: Argon2id password hashing, BLAKE3 token/session hashing, rate-limiting, CSP, HSTS, and non-enumerating authentication.
- **Multi-Tenant & RBAC**: Tenant isolation, fine-grained permission matrix, role assignments, and organizational user groups.
- **Personal Access Tokens & Service Accounts**: Machine-to-machine authentication with automatic prefix tracking and instant revocation.
- **Embedded WebAssembly UI**: Dioxus-powered administration dashboard with `#boot-loader` lifecycle management.
- **Comprehensive CLI Tooling**: Automated `init`, `doctor`, `migrate`, `backup`, `restore`, and user management commands.

---

## Quickstart

```bash
# Initialize application directory, configuration, and default administrator
nx9-auth init

# Verify installation & system health
nx9-auth doctor

# Start server
nx9-auth serve
```

---

## Configuration

Configure `config.toml` or set environment variables:

```toml
[server]
host = "127.0.0.1"
port = 8655
production = false
cookie_secure = false

[database]
# SQLite URL or file path:
url = "sqlite://./data/auth.db?mode=rwc"

# Or enterprise PostgreSQL:
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

---

## Documentation Index

- [Runtime Lifecycle & Graceful Shutdown](docs/runtime-lifecycle.md)
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

## License

Dual-licensed under either of:
- Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)

at your option.
