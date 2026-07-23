-- ── Add Application Credentials Columns & Permissions (PostgreSQL) ───────────

ALTER TABLE applications ADD COLUMN IF NOT EXISTS client_id TEXT;
ALTER TABLE applications ADD COLUMN IF NOT EXISTS description TEXT;
ALTER TABLE applications ADD COLUMN IF NOT EXISTS client_secret_hash TEXT;
ALTER TABLE applications ADD COLUMN IF NOT EXISTS redirect_uris TEXT;
ALTER TABLE applications ADD COLUMN IF NOT EXISTS scopes TEXT;

-- Backfill client_id for existing applications
UPDATE applications SET client_id = 'nx9_app_' || replace(id, '-', '') WHERE client_id IS NULL;

-- Create unique index on client_id
CREATE UNIQUE INDEX IF NOT EXISTS idx_applications_client_id ON applications(client_id);

-- Seed applications:manage permission
INSERT INTO permissions (id, name, description) VALUES
    ('20000000-0000-0000-0000-000000000008', 'applications:manage', 'Manage registered application credentials')
ON CONFLICT (name) DO NOTHING;

-- Grant permission to admin role
INSERT INTO role_permissions (role_id, permission_id) VALUES
    ('10000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000008')
ON CONFLICT DO NOTHING;
