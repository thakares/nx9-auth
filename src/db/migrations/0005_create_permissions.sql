CREATE TABLE IF NOT EXISTS permissions (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL UNIQUE,
    description TEXT
);
