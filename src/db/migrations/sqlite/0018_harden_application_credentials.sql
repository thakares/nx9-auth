-- ── Application Credentials Production Hardening (SQLite) ───────────────────

-- Backfill any remaining applications with client_id if missing
UPDATE applications SET client_id = 'nx9_app_' || replace(id, '-', '') WHERE client_id IS NULL;

-- Ensure unique index on client_id exists
CREATE UNIQUE INDEX IF NOT EXISTS idx_applications_client_id ON applications(client_id);
