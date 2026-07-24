-- Application membership: assign existing NX9-Auth users to registered applications.
-- Membership roles (owner/admin/member) are lightweight metadata only and do not
-- grant global RBAC permissions such as applications:manage.

CREATE TABLE IF NOT EXISTS application_members (
    id              TEXT    PRIMARY KEY NOT NULL,
    application_id  TEXT    NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    user_id         TEXT    NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT    NOT NULL DEFAULT 'member'
                        CHECK (role IN ('owner', 'admin', 'member')),
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (application_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_application_members_application
    ON application_members(application_id);

CREATE INDEX IF NOT EXISTS idx_application_members_user
    ON application_members(user_id);
