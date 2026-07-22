CREATE TABLE IF NOT EXISTS service_accounts (
    id          TEXT    PRIMARY KEY NOT NULL,
    tenant_id   TEXT    NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name        TEXT    NOT NULL,
    description TEXT,
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT    NOT NULL DEFAULT (to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z\')),
    updated_at  TEXT    NOT NULL DEFAULT (to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z\')),
    UNIQUE (tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_service_accounts_tenant ON service_accounts(tenant_id);
