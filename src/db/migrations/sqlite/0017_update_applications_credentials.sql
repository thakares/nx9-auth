-- ── Add Application Credentials Columns & Permissions (SQLite) ────────────────

ALTER TABLE applications ADD COLUMN client_id TEXT;
ALTER TABLE applications ADD COLUMN description TEXT;
ALTER TABLE applications ADD COLUMN client_secret_hash TEXT;
ALTER TABLE applications ADD COLUMN redirect_uris TEXT;
ALTER TABLE applications ADD COLUMN scopes TEXT;

-- Backfill client_id for existing applications
UPDATE applications SET client_id = 'nx9_app_' || replace(id, '-', '') WHERE client_id IS NULL;

-- Create unique index on client_id
CREATE UNIQUE INDEX IF NOT EXISTS idx_applications_client_id ON applications(client_id);

-- Seed applications:manage permission
INSERT OR IGNORE INTO permissions (id, name, description) VALUES
    ('20000000-0000-0000-0000-000000000008', 'applications:manage', 'Manage registered application credentials');

-- Grant permission to admin role
INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES
    ('10000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000008');
