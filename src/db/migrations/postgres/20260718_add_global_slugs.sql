-- nx9-auth: Global Slugs implementation (PostgreSQL)
-- A unified registry for slugs across all resources (tenant, user, group, role, app, etc.)
-- Ensures global uniqueness and immutable references.

CREATE TABLE IF NOT EXISTS global_slugs (
    slug            TEXT    PRIMARY KEY NOT NULL,
    entity_type     TEXT    NOT NULL, -- 'tenant', 'user', 'role', 'group', 'permission', 'application', 'service_account', 'organization', 'team'
    entity_id       TEXT    NOT NULL,
    tenant_id       TEXT    NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    created_at      TEXT    NOT NULL DEFAULT (to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"'))
);

CREATE INDEX IF NOT EXISTS idx_global_slugs_entity ON global_slugs(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_global_slugs_tenant ON global_slugs(tenant_id);

-- Add slug column to existing tables for quick lookup and joins
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS slug TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS slug TEXT;
ALTER TABLE roles ADD COLUMN IF NOT EXISTS slug TEXT;
ALTER TABLE permissions ADD COLUMN IF NOT EXISTS slug TEXT;
ALTER TABLE applications ADD COLUMN IF NOT EXISTS slug TEXT;
ALTER TABLE service_accounts ADD COLUMN IF NOT EXISTS slug TEXT;

-- Backfill basic slugs:
UPDATE tenants SET slug = lower(replace(name, ' ', '-')) WHERE slug IS NULL;
UPDATE users SET slug = lower(username) WHERE slug IS NULL;
UPDATE roles SET slug = lower(replace(name, ' ', '-')) WHERE slug IS NULL;
UPDATE permissions SET slug = lower(replace(name, ' ', '-')) WHERE slug IS NULL;
UPDATE applications SET slug = lower(replace(name, ' ', '-')) WHERE slug IS NULL;
UPDATE service_accounts SET slug = lower(replace(name, ' ', '-')) WHERE slug IS NULL;

-- Insert backfilled slugs into registry
INSERT INTO global_slugs (slug, entity_type, entity_id, tenant_id)
SELECT slug, 'tenant', id, id FROM tenants WHERE slug IS NOT NULL
ON CONFLICT DO NOTHING;

INSERT INTO global_slugs (slug, entity_type, entity_id, tenant_id)
SELECT slug, 'user', id, tenant_id FROM users WHERE slug IS NOT NULL
ON CONFLICT DO NOTHING;

INSERT INTO global_slugs (slug, entity_type, entity_id, tenant_id)
SELECT slug, 'role', id, '00000000-0000-0000-0000-000000000001' FROM roles WHERE slug IS NOT NULL
ON CONFLICT DO NOTHING;

INSERT INTO global_slugs (slug, entity_type, entity_id, tenant_id)
SELECT slug, 'permission', id, '00000000-0000-0000-0000-000000000001' FROM permissions WHERE slug IS NOT NULL
ON CONFLICT DO NOTHING;

INSERT INTO global_slugs (slug, entity_type, entity_id, tenant_id)
SELECT slug, 'application', id, tenant_id FROM applications WHERE slug IS NOT NULL
ON CONFLICT DO NOTHING;

INSERT INTO global_slugs (slug, entity_type, entity_id, tenant_id)
SELECT slug, 'service_account', id, tenant_id FROM service_accounts WHERE slug IS NOT NULL
ON CONFLICT DO NOTHING;
