CREATE TABLE IF NOT EXISTS service_accounts (
    id          TEXT    PRIMARY KEY NOT NULL,
    tenant_id   TEXT    NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name        TEXT    NOT NULL,
    description TEXT,
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_service_accounts_tenant ON service_accounts(tenant_id);
