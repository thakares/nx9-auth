-- ── Roles ────────────────────────────────────────────────────────────────────

INSERT INTO roles (id, name, description) VALUES
    ('10000000-0000-0000-0000-000000000001', 'admin',  'Full system access'),
    ('10000000-0000-0000-0000-000000000002', 'editor', 'Can manage content and users'),
    ('10000000-0000-0000-0000-000000000003', 'viewer', 'Read-only access')
ON CONFLICT DO NOTHING;

-- ── Permissions ───────────────────────────────────────────────────────────────

INSERT INTO permissions (id, name, description) VALUES
    ('20000000-0000-0000-0000-000000000001', 'users:create',  'Create new user accounts'),
    ('20000000-0000-0000-0000-000000000002', 'users:update',  'Update user accounts'),
    ('20000000-0000-0000-0000-000000000003', 'users:delete',  'Disable user accounts'),
    ('20000000-0000-0000-0000-000000000004', 'tokens:create', 'Create API tokens for any user'),
    ('20000000-0000-0000-0000-000000000005', 'tokens:revoke', 'Revoke API tokens for any user'),
    ('20000000-0000-0000-0000-000000000006', 'roles:manage',  'Assign and revoke roles'),
    ('20000000-0000-0000-0000-000000000007', 'audit:view',    'View audit log entries')
ON CONFLICT DO NOTHING;

-- ── Admin role gets all permissions ──────────────────────────────────────────

INSERT INTO role_permissions (role_id, permission_id)
SELECT '10000000-0000-0000-0000-000000000001', id FROM permissions
ON CONFLICT DO NOTHING;

-- ── Editor role permissions ───────────────────────────────────────────────────

INSERT INTO role_permissions (role_id, permission_id) VALUES
    ('10000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000001'),
    ('10000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000002')
ON CONFLICT DO NOTHING;

-- ── Default applications ──────────────────────────────────────────────────────

INSERT INTO applications (id, tenant_id, name, slug, enabled) VALUES
    ('30000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', 'BZOD',         'bzod',         1),
    ('30000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000001', 'ChronoSeal',   'chronoseal',   1),
    ('30000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000001', 'nx9-dns',      'nx9-dns',      1)
ON CONFLICT DO NOTHING;
