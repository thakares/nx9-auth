# nx9-auth Deployment Guide

This guide describes how to deploy and upgrade `nx9-auth` on production systems.

## Prerequisites

- Debian- or Ubuntu-compatible Linux system.
- `systemd` init system.
- Root or sudo privileges.
- Pre-compiled `nx9-auth` release binary (get it from the release package `dist/nx9-auth`).

---

## 1. Fresh Installation

To install `nx9-auth` as a systemd service, run the `deploy.sh` script with root privileges:

```bash
sudo bash deploy.sh /path/to/compiled/nx9-auth
```

This script will automatically:
1. Create a dedicated system user `nx9-auth`.
2. Setup system directories:
   - Config directory: `/etc/nx9-auth/`
   - Data directory: `/var/lib/nx9-auth/`
   - Logs directory: `/var/log/nx9-auth/`
3. Copy the binary to `/usr/local/bin/nx9-auth`.
4. Generate a default configuration file `/etc/nx9-auth/config.toml` (if not already present).
5. Install and configure a hardened systemd service file `/etc/systemd/system/nx9-auth.service`.
6. Run database migrations.
7. Start the service.
8. Execute diagnostic check (`doctor` command).

---

## 2. Configuration

Modify `/etc/nx9-auth/config.toml` to customize settings.

```toml
[server]
host = "127.0.0.1"
port = 8655

[database]
path = "/var/lib/nx9-auth/auth.db"

[security]
session_ttl_hours = 24
session_absolute_ttl_days = 30
token_ttl_days = 365
argon2_memory = 65536
argon2_iterations = 3
argon2_parallelism = 1

[audit]
enabled = true
```

After modifying the configuration, restart the service:
```bash
sudo systemctl restart nx9-auth
```

---

## 3. Initial Setup

Once the service is deployed, create your first administrative user:

```bash
sudo -u nx9-auth nx9-auth create-admin my-admin-username --config /etc/nx9-auth/config.toml
```

---

## 4. Upgrade Installation

Upgrading `nx9-auth` is safe and preserves both the configuration and the database.

1. Stop the active service:
   ```bash
   sudo systemctl stop nx9-auth
   ```
2. Run the `deploy.sh` script pointing to the new binary:
   ```bash
   sudo bash deploy.sh /path/to/new/nx9-auth
   ```
   *Note: Since the configuration file and database already exist, the deploy script will skip creating them, safely leaving existing user accounts, sessions, and logs untouched.*
3. Verify the deployment:
   ```bash
   sudo -u nx9-auth nx9-auth doctor --config /etc/nx9-auth/config.toml
   ```
