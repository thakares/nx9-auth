-- Seed the default tenant.
-- Uses INSERT OR IGNORE so re-running migrations is safe.
INSERT OR IGNORE INTO tenants (id, name, slug, enabled)
VALUES ('00000000-0000-0000-0000-000000000001', 'Default', 'default', 1);
