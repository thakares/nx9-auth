# nx9-auth

<p align="center">

**Enterprise Identity & Access Management (IAM)**

*Self-Hosted • Privacy-First • Pure Rust • Single Binary • Linux Native*

[![Version](https://img.shields.io/badge/version-v0.2.0-blue.svg)]()
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux-success.svg)]()
[![SQLite](https://img.shields.io/badge/database-SQLite-blue.svg)]()
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-coming%20soon-lightgrey.svg)]()

</p>

---

## Overview

**nx9-auth** is a modern, enterprise-grade Identity & Access Management (IAM) platform built entirely in Rust.

It provides centralized authentication, authorization, user administration, multi-tenancy, session management, audit logging and administrative tools in a single deployable application.

Unlike traditional IAM platforms that require multiple services, Java application servers, Redis, PostgreSQL, Kubernetes and extensive operational overhead, **nx9-auth** is intentionally designed around simplicity, security and complete ownership.

Current release **v0.2.0** delivers a production-quality Phase 0 implementation using SQLite with a modern Dioxus WebAssembly administration interface.

---

# Why nx9-auth?

Modern identity platforms are often:

- Complex
- Heavyweight
- Cloud dependent
- Expensive
- Difficult to self-host

nx9-auth follows a different philosophy.

### Design Goals

- Self-hosted first
- Privacy first
- Linux native
- Pure Rust
- Single executable
- Minimal dependencies
- Enterprise security
- Zero vendor lock-in
- Open source forever

---

# Features

## Identity

- User Management
- User Profiles
- Password Authentication
- Password Reset
- Account Locking
- Profile Management

---

## Authorization

- Role Based Access Control (RBAC)
- Permissions
- Multiple Roles per User
- Fine-grained Authorization
- Authorization Middleware

---

## Multi-Tenancy

- Tenant Management
- Tenant Isolation
- Tenant Administration

---

## Organization

- Groups
- Applications
- Service Accounts

---

## Security

- Secure Sessions
- API Tokens
- Secure Authentication
- Security Headers
- Audit Logging
- Password Hashing (Argon2id)
- Cookie Authentication

---

## Administration

- Dashboard
- User Administration
- Group Administration
- Role Administration
- Permission Administration
- Session Administration
- Application Administration
- Service Account Administration
- Tenant Administration
- Audit Viewer
- Profile Settings

---

## User Interface

- Dioxus WebAssembly UI
- Responsive Design
- Enterprise Dashboard
- Modern Navigation
- Dark Theme

---

# Screenshots

*(Coming with future releases)*

- Login
- Dashboard
- Users
- Roles
- Permissions
- Audit Log
- Sessions
- Applications

---

# Architecture

```
                   Browser
                      │
              Dioxus WebAssembly
                      │
                 Axum HTTP Server
                      │
              Authentication Layer
                      │
              Authorization Layer
                      │
                 REST API Layer
                      │
             Database Provider API
                      │
             SQLite Repository Layer
                      │
                  SQLite Database
```

---

# Technology Stack

| Component | Technology |
|------------|------------|
| Language | Rust 2021 |
| Backend | Axum |
| Frontend | Dioxus |
| UI Runtime | WebAssembly |
| Async Runtime | Tokio |
| Database | SQLite |
| SQL Layer | SQLx |
| Serialization | Serde |
| Password Hashing | Argon2id |
| Configuration | TOML |

---

# Current Capabilities

| Module | Status |
|----------|--------|
| Dashboard | ✅ |
| Authentication | ✅ |
| Users | ✅ |
| Roles | ✅ |
| Permissions | ✅ |
| Groups | ✅ |
| Tenants | ✅ |
| Applications | ✅ |
| Sessions | ✅ |
| API Tokens | ✅ |
| Service Accounts | ✅ |
| Audit Logs | ✅ |
| Profile | ✅ |
| SQLite | ✅ |
| PostgreSQL | 🚧 |
| OAuth2 | 🚧 |
| OIDC | 🚧 |
| SAML | 🚧 |

---

# REST API

```
/api/v1/auth
/api/v1/dashboard
/api/v1/users
/api/v1/groups
/api/v1/roles
/api/v1/permissions
/api/v1/tenants
/api/v1/applications
/api/v1/service-accounts
/api/v1/tokens
/api/v1/sessions
/api/v1/audit
/api/v1/profile
```

---

# Installation

Clone the repository

```bash
git clone https://github.com/thakares/nx9-auth.git
cd nx9-auth
```

Build

```bash
cargo build --release
```

Initialize

```bash
./target/release/nx9-auth init
```

Configure

```bash
cp config.example.toml config.toml
```

Run

```bash
./target/release/nx9-auth serve
```

The administration interface will be available after startup.

---

# CLI

```
nx9-auth init
nx9-auth setup
nx9-auth migrate
nx9-auth serve
nx9-auth doctor
nx9-auth version
```

---

# Configuration

Configuration is stored in

```
config.toml
```

An example configuration is available in

```
config.example.toml
```

---

# Project Layout

```
src/
├── api/
├── audit/
├── cli/
├── config/
├── db/
│   ├── migrations/
│   ├── models/
│   ├── repository/
│   │   ├── sqlite/
│   │   ├── postgres/
│   │   └── traits.rs
│   └── provider.rs
├── identity/
├── middleware/
├── security/
├── state.rs
└── main.rs

ui/
├── assets/
├── components/
├── layouts/
├── pages/
└── services/

tests/

docs/
```

---

# Security

Security is a fundamental design goal.

Implemented protections include:

- Argon2id password hashing
- Secure session management
- Secure API tokens
- Audit logging
- RBAC
- Tenant isolation
- Security headers
- Authorization middleware
- Authentication middleware

Passwords are never stored in plaintext.

---

# Development

Format

```bash
cargo fmt
```

Check

```bash
cargo check
```

Lint

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Tests

```bash
cargo test
```

Build UI

```bash
scripts/build-ui.sh
```

---

# Roadmap

## Phase 0 ✅

- Enterprise IAM
- SQLite
- Web Administration
- REST API
- RBAC
- Multi-tenancy

---

## Phase 1

- PostgreSQL
- Database abstraction improvements
- Performance tuning

---

## Phase 2

- OAuth2
- OpenID Connect
- SAML
- Multi-factor Authentication
- WebAuthn / Passkeys

---

## Phase 3

- Redis
- High Availability
- Clustering
- Distributed Sessions

---

## Phase 4

- LDAP
- Active Directory
- SCIM
- Enterprise Federation

---

# Documentation

Additional documentation is available in the `docs/` directory.

- Authentication
- Deployment
- Architecture
- API Reference
- Development Guide

---

# Contributing

Contributions are welcome.

Please ensure every contribution:

```bash
cargo fmt
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test
```

passes before opening a pull request.

---

# License

Released under the MIT License.

See the LICENSE file for details.

---

# About NX9

**nx9-auth** is part of the **NX9** ecosystem.

NX9 is a collection of self-hosted, privacy-first, Linux-native infrastructure software written entirely in Rust.

## NX9 Principles

- Self-hosted First
- Privacy First
- Linux Native
- Pure Rust
- Single Binary
- Minimal Dependencies
- Open Standards
- Enterprise Security
- FOSS Forever

---

<p align="center">

**Own your infrastructure. Own your identity. Own your data.**

**No subscriptions. No vendor lock-in. No compromises.**

</p>