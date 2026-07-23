-- ── Application Credentials Production Hardening (PostgreSQL) ───────────────

-- Backfill any remaining applications with client_id if missing
UPDATE applications SET client_id = 'nx9_app_' || replace(id, '-', '') WHERE client_id IS NULL;

-- Enforce NOT NULL constraint on client_id
ALTER TABLE applications ALTER COLUMN client_id SET NOT NULL;

-- Ensure unique index on client_id exists
CREATE UNIQUE INDEX IF NOT EXISTS idx_applications_client_id ON applications(client_id);
