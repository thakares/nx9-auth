CREATE TABLE IF NOT EXISTS user_profiles (
    user_id       TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email         TEXT,
    full_name     TEXT,
    avatar_url    TEXT,
    metadata_json TEXT
);
