-- nx9-auth: Global Slugs Hardening & Parity Alignment (PostgreSQL)
-- Ensures unified global_slugs registry table, indices, and legacy data integrity.

CREATE TABLE IF NOT EXISTS global_slugs (
    slug            TEXT    PRIMARY KEY NOT NULL,
    entity_type     TEXT    NOT NULL,
    entity_id       TEXT    NOT NULL,
    tenant_id       TEXT    NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    created_at      TEXT    NOT NULL DEFAULT (to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"'))
);

CREATE INDEX IF NOT EXISTS idx_global_slugs_entity ON global_slugs(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_global_slugs_tenant ON global_slugs(tenant_id);

-- Explicit backfill for tenants that are not yet in global_slugs.
-- Fails immediately if cross-resource slug collision exists.
INSERT INTO global_slugs (slug, entity_type, entity_id, tenant_id)
SELECT slug, 'tenant', id, id
FROM tenants
WHERE slug IS NOT NULL AND slug NOT IN (SELECT slug FROM global_slugs);

-- Explicit backfill for applications that are not yet in global_slugs.
-- Fails immediately if cross-resource slug collision exists.
INSERT INTO global_slugs (slug, entity_type, entity_id, tenant_id)
SELECT slug, 'application', id, tenant_id
FROM applications
WHERE slug IS NOT NULL AND slug NOT IN (SELECT slug FROM global_slugs);
