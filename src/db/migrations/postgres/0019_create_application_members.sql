-- Application membership: assign existing NX9-Auth users to registered applications.
-- Membership roles (owner/admin/member) are lightweight metadata only and do not
-- grant global RBAC permissions such as applications:manage.

CREATE TABLE IF NOT EXISTS application_members (
    id              TEXT    PRIMARY KEY NOT NULL,
    application_id  TEXT    NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    user_id         TEXT    NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT    NOT NULL DEFAULT 'member'
                        CHECK (role IN ('owner', 'admin', 'member')),
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TEXT    NOT NULL DEFAULT (to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')),
    updated_at      TEXT    NOT NULL DEFAULT (to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')),
    UNIQUE (application_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_application_members_application
    ON application_members(application_id);

CREATE INDEX IF NOT EXISTS idx_application_members_user
    ON application_members(user_id);
