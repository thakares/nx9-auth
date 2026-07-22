#!/usr/bin/env bash
# deploy.sh — nx9-auth installer for Debian/Ubuntu systems
#
# Usage: sudo bash deploy.sh [path/to/nx9-auth-binary]
# Requires: root, systemd

set -euo pipefail

BINARY_PATH="${1:-./target/release/nx9-auth}"
SERVICE_USER="nx9-auth"
INSTALL_BIN="/usr/local/bin/nx9-auth"
CONFIG_DIR="/etc/nx9-auth"
DATA_DIR="/var/lib/nx9-auth"
LOG_DIR="/var/log/nx9-auth"
SERVICE_FILE="/etc/systemd/system/nx9-auth.service"

# ── Colours ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
ok()   { echo -e "${GREEN}  ✓${NC}  $*"; }
warn() { echo -e "${YELLOW}  !${NC}  $*"; }
fail() { echo -e "${RED}  ✗${NC}  $*"; exit 1; }

# ── Prerequisites ─────────────────────────────────────────────────────────────
[[ $EUID -eq 0 ]] || fail "This script must be run as root."
[[ -f "$BINARY_PATH" ]] || fail "Binary not found at: $BINARY_PATH — build with 'cargo build --release' first."

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "   nx9-auth deploy"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# ── Create system user ────────────────────────────────────────────────────────
if id -u "$SERVICE_USER" &>/dev/null; then
    warn "System user '$SERVICE_USER' already exists — skipping creation."
else
    useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
    ok "Created system user: $SERVICE_USER"
fi

# ── Create directories ────────────────────────────────────────────────────────
for dir in "$CONFIG_DIR" "$DATA_DIR" "$LOG_DIR"; do
    mkdir -p "$dir"
    chown "$SERVICE_USER:$SERVICE_USER" "$dir"
    chmod 750 "$dir"
done
ok "Directories created: $CONFIG_DIR, $DATA_DIR, $LOG_DIR"

# ── Install binary ────────────────────────────────────────────────────────────
cp "$BINARY_PATH" "$INSTALL_BIN"
chmod 755 "$INSTALL_BIN"
ok "Binary installed: $INSTALL_BIN"

# ── Write default config if not present ──────────────────────────────────────
if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
    cat > "$CONFIG_DIR/config.toml" <<'EOF'
[server]
host = "0.0.0.0"
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
EOF
    chown root:"$SERVICE_USER" "$CONFIG_DIR/config.toml"
    chmod 640 "$CONFIG_DIR/config.toml"
    ok "Default config written: $CONFIG_DIR/config.toml"
else
    warn "Config already exists — skipping: $CONFIG_DIR/config.toml"
fi

# ── Install systemd service ───────────────────────────────────────────────────
cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=nx9-auth Identity and Access Management Service
Documentation=https://github.com/nx9/nx9-auth
After=network.target
Wants=network.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
ExecStart=$INSTALL_BIN serve --config $CONFIG_DIR/config.toml
Restart=on-failure
RestartSec=5s
TimeoutStopSec=10s

# Security hardening
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
NoNewPrivileges=true
CapabilityBoundingSet=
AmbientCapabilities=
LockPersonality=true
MemoryDenyWriteExecute=true
PrivateDevices=true
ProtectClock=true
ProtectControlGroups=true
ProtectHostname=true
ProtectKernelLogs=true
ProtectKernelModules=true
ProtectKernelTunables=true
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
RestrictNamespaces=true
RestrictRealtime=true
SystemCallArchitectures=native
SystemCallFilter=@system-service

# Writable paths
ReadWritePaths=$DATA_DIR $LOG_DIR

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=nx9-auth

[Install]
WantedBy=multi-user.target
EOF

chmod 644 "$SERVICE_FILE"
ok "Systemd service installed: $SERVICE_FILE"

# ── Initialize database and configuration ─────────────────────────────────────
echo ""
echo "Initializing database and configuration..."
sudo -u "$SERVICE_USER" "$INSTALL_BIN" init --config "$CONFIG_DIR/config.toml" --non-interactive --skip-admin
ok "Initialization complete"

# ── Enable and start service ──────────────────────────────────────────────────
systemctl daemon-reload
systemctl enable nx9-auth
systemctl restart nx9-auth
ok "nx9-auth service enabled and started"

# ── Doctor check ──────────────────────────────────────────────────────────────
echo ""
sleep 2  # Brief wait for service to start
sudo -u "$SERVICE_USER" "$INSTALL_BIN" doctor --config "$CONFIG_DIR/config.toml" || true

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "   nx9-auth deployed successfully!"
echo ""
echo "   Service:  systemctl status nx9-auth"
echo "   Logs:     journalctl -u nx9-auth -f"
echo "   Config:   $CONFIG_DIR/config.toml"
echo "   Database: $DATA_DIR/auth.db"
echo ""
echo "   Next step:"
echo "   Create your first administrator account:"
echo "   sudo -u nx9-auth nx9-auth init --config $CONFIG_DIR/config.toml"
echo ""
echo "   Then verify:"
echo "   systemctl status nx9-auth"
echo "   curl http://127.0.0.1:8655/health"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
