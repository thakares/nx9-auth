CREATE TABLE IF NOT EXISTS users (
    id              TEXT    PRIMARY KEY NOT NULL,
    tenant_id       TEXT    NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    username        TEXT    NOT NULL,
    password_hash   TEXT    NOT NULL,
    -- 1 = active, 2 = disabled, 3 = locked
    status          INTEGER NOT NULL DEFAULT 1,
    last_login_at   TEXT,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (tenant_id, username)
);

CREATE INDEX IF NOT EXISTS idx_users_username  ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_tenant_id ON users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_users_status    ON users(status);
