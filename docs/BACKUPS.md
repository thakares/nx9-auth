# nx9-auth Database Backups & Recovery

Since `nx9-auth` uses SQLite with Write-Ahead Logging (WAL) enabled, standard file copies of `auth.db` during high-concurrency operations can result in corrupted backups. This guide details the correct procedures for backing up and restoring the database safely.

---

## 1. Online Backups (Recommended)

SQLite provides a built-in backup API that safely reads the database and locks it transactionally to capture a consistent snapshot, merging concurrent WAL journals correctly without interrupting the running service.

To perform an online backup:

```bash
# Create backups directory
mkdir -p /var/backups/nx9-auth

# Run SQLite .backup query
sqlite3 /var/lib/nx9-auth/auth.db ".backup /var/backups/nx9-auth/auth_$(date +%F_%H%M%S).db"

# Change ownership and permissions to protect secrets
chown root:root /var/backups/nx9-auth/auth_*.db
chmod 600 /var/backups/nx9-auth/auth_*.db
```

### Automation via Cron

You can automate this daily by adding a cron job to `/etc/cron.daily/nx9-auth-backup`:

```bash
#!/bin/bash
BACKUP_DIR="/var/backups/nx9-auth"
mkdir -p "$BACKUP_DIR"
sqlite3 /var/lib/nx9-auth/auth.db ".backup $BACKUP_DIR/auth_$(date +%F).db"
chmod 600 "$BACKUP_DIR"/auth_*.db
# Keep only last 30 days of backups
find "$BACKUP_DIR" -name "auth_*.db" -mtime +30 -delete
```

Make sure the cron script is executable:
```bash
chmod +x /etc/cron.daily/nx9-auth-backup
```

---

## 2. Offline Backups

If you need to copy the raw database file directly, you **must** stop the service first to ensure all transactions are fully written to the disk and the WAL log is empty:

```bash
# 1. Stop the service
sudo systemctl stop nx9-auth

# 2. Copy the database file
cp /var/lib/nx9-auth/auth.db /var/backups/nx9-auth/auth_offline_$(date +%F).db

# 3. Start the service
sudo systemctl start nx9-auth
```

---

## 3. Database Recovery

To restore the database from a backup:

```bash
# 1. Stop the running service
sudo systemctl stop nx9-auth

# 2. Backup the current corrupted/old database just in case
mv /var/lib/nx9-auth/auth.db /var/lib/nx9-auth/auth.db.bak

# 3. Copy the backup file into place
cp /var/backups/nx9-auth/auth_2026-06-21.db /var/lib/nx9-auth/auth.db

# 4. Correct ownership and permissions
chown nx9-auth:nx9-auth /var/lib/nx9-auth/auth.db
chmod 640 /var/lib/nx9-auth/auth.db

# 5. Start the service
sudo systemctl start nx9-auth

# 6. Run doctor checks to verify integrity of restored database
sudo -u nx9-auth nx9-auth doctor --config /etc/nx9-auth/config.toml
```
