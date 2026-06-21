# nx9-auth

A lightweight Identity and Access Management (IAM) service.

Built with Rust, Axum, SQLite, and modern security practices, `nx9-auth` provides authentication, authorization, session management, personal access tokens, audit logging, and role-based access control in a single deployable binary.

## Features

* User management
* Role-Based Access Control (RBAC)
* Session authentication
* Personal Access Tokens (PAT)
* Audit logging
* Transaction-safe operations
* SQLite with WAL mode
* Online backups
* Interactive initialization
* Docker and CasaOS support
* Systemd deployment support
* XDG-compliant user mode
* **Dioxus enterprise web UI** (single binary, no Node.js)

## Quick Start

Initialize a new installation:

```bash
nx9-auth init
```

Start the server:

```bash
nx9-auth serve
```

Verify health:

```bash
curl http://127.0.0.1:8655/health
```

Open the UI in a browser:

```text
http://127.0.0.1:8655/
```

## Authentication

Login is **POST-only** with a JSON body (never query parameters):

```bash
curl -sS -X POST http://127.0.0.1:8655/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"your-password"}'
```

Passwords are verified with **Argon2id** and never logged or stored in plaintext.
See [docs/AUTHENTICATION.md](docs/AUTHENTICATION.md) for the full security model.

## Web UI

The management UI is implemented in pure Rust with **Dioxus** (no React/Vue/Node).
It is served from the same process as the REST API.

### Build UI assets

```bash
./scripts/build-ui.sh
```

This compiles `ui/` to WebAssembly and writes static files to `ui/dist/`.
The server serves those files automatically (override path with `NX9_AUTH_UI_DIST`).

### UI features

* Login / logout with session restoration
* Permission-aware sidebar and routing
* User, role, permission, token, application, and service-account management
* Audit log viewer with filters
* Profile and settings (theme: light / dark / system)
* Responsive enterprise shell (header, sidebar, breadcrumbs, toasts)

Frontend RBAC is presentation-only; the backend remains authoritative.

## CLI Commands

```bash
nx9-auth init
nx9-auth serve
nx9-auth doctor

nx9-auth create-user
nx9-auth create-admin

nx9-auth create-token
nx9-auth revoke-token

nx9-auth show-user
nx9-auth show-token

nx9-auth backup
```

## Deployment Modes

### User Mode

Uses XDG directories:

```text
~/.config/nx9-auth/
~/.local/share/nx9-auth/
~/.local/state/nx9-auth/
```

### System Mode

```text
/etc/nx9-auth/
/var/lib/nx9-auth/
/var/log/nx9-auth/
```

### Docker

```bash
docker compose up -d
```

### CasaOS

```text
/DATA/AppData/nx9-auth
├── config
├── db
├── state
└── backups
```

## Security

* Argon2id password hashing
* BLAKE3 token hashing
* Session revocation
* Transactional audit logging
* Timing attack mitigation
* Security regression test suite

## Testing

```bash
cargo test --all
```

Current test coverage includes:

* Unit tests
* Integration tests
* Security tests
* Migration compatibility tests
* CLI tests

## Roadmap

### v0.1.x

* Stable IAM core
* BZOD integration

### v0.2.x

* OAuth2 Authorization Server
* OpenID Connect (OIDC)
* PKCE support

## License

Apache 2.0 or MIT -- Dual License

```
```
