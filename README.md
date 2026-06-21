# nx9-auth

A lightweight Identity and Access Management (IAM) service for the NX9 ecosystem.

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
