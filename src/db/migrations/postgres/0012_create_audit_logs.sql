CREATE TABLE IF NOT EXISTS audit_logs (
    id              TEXT PRIMARY KEY NOT NULL,
    actor_user_id   TEXT REFERENCES users(id) ON DELETE SET NULL,
    target_user_id  TEXT REFERENCES users(id) ON DELETE SET NULL,
    action          TEXT NOT NULL,
    resource_type   TEXT NOT NULL,
    resource_id     TEXT,
    -- 'info', 'warning', 'critical'
    severity        TEXT NOT NULL DEFAULT 'info',
    ip_address      TEXT,
    user_agent      TEXT,
    metadata_json   TEXT,
    created_at      TEXT NOT NULL DEFAULT (to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z\'))
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_actor      ON audit_logs(actor_user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_target     ON audit_logs(target_user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action     ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_logs_severity   ON audit_logs(severity);
