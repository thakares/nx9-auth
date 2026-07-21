CREATE TABLE IF NOT EXISTS tenants (
    id          TEXT    PRIMARY KEY NOT NULL,
    name        TEXT    NOT NULL,
    slug        TEXT    NOT NULL UNIQUE,
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug);
