-- Seed the default tenant.
INSERT INTO tenants (id, name, slug, enabled)
VALUES ('00000000-0000-0000-0000-000000000001', 'Default', 'default', 1)
ON CONFLICT (id) DO NOTHING;
