**License:** Apache-2.0 / MIT Dual License  

---

## Architectural Rationale: Root Docker Manifests

The `Dockerfile`, `docker-compose.yml`, and `compose.casaos.yml` reside at the repository root to comply with standard Docker tooling standards (`docker build .`, `docker compose up`), automated container registry build triggers (Docker Hub, GHCR), and platform app managers (CasaOS, Portainer).

---

## 1. Build the Docker Image

To build the Docker image locally:

```bash
docker build -t nx9-auth:v0.3.0 .
```

---

## 2. Local Development Stack (Docker Compose)

The local stack runs with isolated named volumes to store database and configurations without cluttering host folders:

```yaml
services:
  nx9-auth:
    build: .
    container_name: nx9-auth
    restart: unless-stopped
    ports:
      - "8655:8655"
    volumes:
      - nx9-auth-config:/etc/nx9-auth
      - nx9-auth-db:/var/lib/nx9-auth
      - nx9-auth-state:/var/log/nx9-auth
      - nx9-auth-backups:/var/backups/nx9-auth
    environment:
      - NX9_AUTH_CONFIG=/etc/nx9-auth/config.toml
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:8655/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 15s

volumes:
  nx9-auth-config:
  nx9-auth-db:
  nx9-auth-state:
  nx9-auth-backups:
```

### Steps to Run

1. **Start the service in the background**:
   ```bash
   docker compose up -d
   ```

2. **Initialize config and database** (interactive setup):
   ```bash
   docker exec -it nx9-auth nx9-auth init
   ```
   *Note: If you need to run non-interactively (e.g. in CI), run:*
   ```bash
   docker exec -it nx9-auth nx9-auth init --non-interactive --admin-user admin --admin-password 'YourSecurePasswordHere'
   ```

3. **Check status**:
   Verify the logs or query health check endpoints from the host:
   ```bash
   curl http://127.0.0.1:8655/health
   curl http://127.0.0.1:8655/version
   ```

---

## 3. CasaOS Deployment (Production)

For production deployment on CasaOS, volumes are mapped to the host `/DATA/AppData/nx9-auth` directories:

### Directory Mapping Layout

| Host Path | Container Path | Purpose |
| --- | --- | --- |
| `/DATA/AppData/nx9-auth/config` | `/etc/nx9-auth` | Contains `config.toml` |
| `/DATA/AppData/nx9-auth/db` | `/var/lib/nx9-auth` | Contains `auth.db` |
| `/DATA/AppData/nx9-auth/state` | `/var/log/nx9-auth` | Logs and session files |
| `/DATA/AppData/nx9-auth/backups` | `/var/backups/nx9-auth` | Database snapshots |

### Setup

CasaOS users can import the `compose.casaos.yml` file via the custom install option. After deployment, execute the init flow inside the container:
```bash
docker exec -it nx9-auth nx9-auth init
```

---

## 4. Backups

To trigger a transactionally consistent online SQLite database backup inside the container:
```bash
docker exec -it nx9-auth nx9-auth backup /var/backups/nx9-auth/auth-backup.db
```
The backup will be written directly to `/var/backups/nx9-auth/auth-backup.db` inside the container, which maps to the host's backups directory (e.g. `./backups/` or `/DATA/AppData/nx9-auth/backups/`).

---

## 5. Upgrade

To upgrade the container to a newer release:
```bash
docker compose pull
docker compose up -d
```
